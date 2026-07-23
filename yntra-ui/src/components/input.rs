use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct InputProps {
    #[props(default = String::new())]
    pub placeholder: String,
    pub value: String,
    #[props(optional)]
    pub oninput: EventHandler<FormEvent>,
    #[props(optional)]
    pub onchange: EventHandler<FormEvent>,
    #[props(optional)]
    pub onkeydown: EventHandler<KeyboardEvent>,
    #[props(optional)]
    pub onblur: EventHandler<FocusEvent>,
    #[props(optional)]
    pub onfocus: EventHandler<FocusEvent>,
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    #[props(default = String::new())]
    pub id: String,
    #[props(default = "text".to_string())]
    pub r#type: String,
    #[props(default = String::new())]
    pub aria_label: String,
    #[props(optional)]
    pub aria_describedby: Option<String>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(default = false)]
    pub autofocus: bool,
    #[props(default = String::new())]
    pub name: String,
    #[props(default = String::new())]
    pub step: String,
    #[props(default = String::new())]
    pub min: String,
    #[props(default = String::new())]
    pub max: String,
    #[props(optional)]
    pub error: Option<String>,
    #[props(default = false)]
    pub is_invalid: bool,
    #[props(default = String::new())]
    pub autocomplete: String,
    #[props(default = String::new())]
    pub maxlength: String,
    #[props(default = String::new())]
    pub pattern: String,
}

#[component]
pub fn Input(props: InputProps) -> Element {
    let mut resolved_style = props.style.clone();
    let is_invalid = props.is_invalid || props.error.is_some();
    let invalid_cls = if is_invalid { "is-invalid" } else { "" };
    
    // Auto-detect tailwind padding-left classes to prevent override by default shorthand padding
    if props.class.contains("pl-9") && !resolved_style.contains("padding-left") {
        if !resolved_style.is_empty() && !resolved_style.ends_with(';') {
            resolved_style.push(';');
        }
        resolved_style.push_str("padding-left: 2.25rem;");
    }

    rsx! {
        input {
            class: "yntra-input {props.class} {invalid_cls}",
            style: "{resolved_style}",
            r#type: "{props.r#type}",
            placeholder: "{props.placeholder}",
            value: "{props.value}",
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            autofocus: props.autofocus,
            name: if props.name.is_empty() { None } else { Some(props.name.clone()) },
            id: if props.id.is_empty() { None } else { Some(props.id.clone()) },
            aria_label: if props.aria_label.is_empty() { None } else { Some(props.aria_label.clone()) },
            aria_describedby: props.aria_describedby.as_ref().filter(|s| !s.is_empty()).cloned(),
            aria_invalid: if is_invalid { "true" } else { "false" },
            step: if props.step.is_empty() { None } else { Some(props.step.clone()) },
            min: if props.min.is_empty() { None } else { Some(props.min.clone()) },
            max: if props.max.is_empty() { None } else { Some(props.max.clone()) },
            autocomplete: if props.autocomplete.is_empty() { None } else { Some(props.autocomplete.clone()) },
            maxlength: if props.maxlength.is_empty() { None } else { Some(props.maxlength.clone()) },
            pattern: if props.pattern.is_empty() { None } else { Some(props.pattern.clone()) },
            oninput: move |evt| props.oninput.call(evt),
            onchange: move |evt| props.onchange.call(evt),
            onkeydown: move |evt| props.onkeydown.call(evt),
            onblur: move |evt| props.onblur.call(evt),
            onfocus: move |evt| props.onfocus.call(evt),
        }
        style {
            r#"
            :where(.yntra-input) {{
                background: rgba(0, 0, 0, 0.2);
                border: 1px solid var(--border-color);
                color: var(--text-primary);
                padding: 0.65rem 0.85rem;
                border-radius: 8px;
                font-size: 0.9rem;
                outline: none;
                transition: border-color 0.2s, box-shadow 0.2s;
                width: 100%;
                box-sizing: border-box;
            }}
            :where(.yntra-input):focus {{
                border-color: var(--accent-color);
            }}
            :where(.yntra-input.is-invalid) {{
                border-color: rgba(239, 68, 68, 0.8) !important;
            }}
            :where(.yntra-input.is-invalid):focus {{
                box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.3) !important;
            }}
            "#
        }
    }
}
