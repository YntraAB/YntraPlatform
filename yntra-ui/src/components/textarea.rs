use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct TextAreaProps {
    #[props(default = String::new())]
    pub placeholder: String,
    pub value: String,
    pub oninput: EventHandler<FormEvent>,
    #[props(default = 4)]
    pub rows: u32,
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
pub fn TextArea(props: TextAreaProps) -> Element {
    let is_invalid = props.is_invalid || props.error.is_some();
    let invalid_cls = if is_invalid { "is-invalid" } else { "" };
    rsx! {
        textarea {
            class: "yntra-textarea {props.class} {invalid_cls}",
            id: if props.id.is_empty() { None } else { Some(props.id.clone()) },
            aria_describedby: props.aria_describedby.as_ref().filter(|s| !s.is_empty()).cloned(),
            aria_invalid: if is_invalid { "true" } else { "false" },
            placeholder: "{props.placeholder}",
            value: "{props.value}",
            rows: "{props.rows}",
            oninput: move |evt| props.oninput.call(evt),
            style {
                r#"
                .yntra-textarea {{
                    background: var(--primary-color-3);
                    border: 1px solid var(--primary-color-6);
                    border-radius: 10px;
                    padding: 0.8rem 1rem;
                    color: var(--secondary-color-2);
                    font-size: 1rem;
                    outline: none;
                    transition: border-color 0.2s, box-shadow 0.2s;
                    width: 100%;
                    box-sizing: border-box;
                    resize: vertical;
                    font-family: inherit;
                }}
                .yntra-textarea:focus {{
                    border-color: var(--focused-border-color);
                    box-shadow: 0 0 0 2px rgba(43, 127, 255, 0.2);
                }}
                .yntra-textarea.is-invalid {{
                    border-color: rgba(239, 68, 68, 0.8) !important;
                }}
                .yntra-textarea.is-invalid:focus {{
                    box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.3) !important;
                }}
                "#
            }
        }
    }
}
