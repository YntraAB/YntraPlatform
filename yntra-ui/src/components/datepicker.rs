use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct DatePickerProps {
    pub value: String,
    pub onchange: EventHandler<FormEvent>,
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub id: String,
    #[props(optional)]
    pub aria_describedby: Option<String>,
    #[props(optional)]
    pub error: Option<String>,
    #[props(default = false)]
    pub is_invalid: bool,
}

#[component]
pub fn DatePicker(props: DatePickerProps) -> Element {
    let is_invalid = props.is_invalid || props.error.is_some();
    let invalid_cls = if is_invalid { "is-invalid" } else { "" };
    rsx! {
        input {
            class: "yntra-datepicker {props.class} {invalid_cls}",
            id: if props.id.is_empty() { None } else { Some(props.id.clone()) },
            aria_describedby: props.aria_describedby.as_ref().filter(|s| !s.is_empty()).cloned(),
            aria_invalid: if is_invalid { "true" } else { "false" },
            r#type: "date",
            value: "{props.value}",
            onchange: move |evt| props.onchange.call(evt),
            style {
                r#"
                .yntra-datepicker {{
                    background: rgba(255, 255, 255, 0.05);
                    border: 1px solid rgba(255, 255, 255, 0.08);
                    border-radius: 10px;
                    padding: 0.75rem 1rem;
                    color: hsl(220, 14.3%, 95.9%);
                    font-size: 1rem;
                    outline: none;
                    transition: border-color 0.2s, box-shadow 0.2s;
                    box-sizing: border-box;
                    font-family: inherit;
                    color-scheme: dark;
                }}
                .yntra-datepicker:focus {{
                    border-color: var(--focused-border-color);
                    box-shadow: 0 0 0 2px var(--accent-color-soft);
                }}
                .yntra-datepicker.is-invalid {{
                    border-color: rgba(239, 68, 68, 0.8) !important;
                }}
                .yntra-datepicker.is-invalid:focus {{
                    box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.3) !important;
                }}
                "#
            }
        }
    }
}
