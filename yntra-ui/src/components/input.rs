use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct InputProps {
    #[props(default = String::new())]
    pub placeholder: String,
    pub value: String,
    pub oninput: EventHandler<FormEvent>,
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub id: String,
    #[props(default = "text".to_string())]
    pub r#type: String,
    #[props(default = String::new())]
    pub aria_label: String,
    #[props(default = String::new())]
    pub aria_describedby: String,
}

#[component]
pub fn Input(props: InputProps) -> Element {
    rsx! {
        input {
            class: "yntra-input {props.class}",
            r#type: "{props.r#type}",
            placeholder: "{props.placeholder}",
            value: "{props.value}",
            oninput: move |evt| props.oninput.call(evt),
            id: if props.id.is_empty() { None } else { Some(props.id.clone()) },
            aria_label: if props.aria_label.is_empty() { None } else { Some(props.aria_label.clone()) },
            aria_describedby: if props.aria_describedby.is_empty() { None } else { Some(props.aria_describedby.clone()) },
            style {
                r#"
                .yntra-input {{
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
                }}
                .yntra-input:focus {{
                    border-color: var(--focused-border-color);
                    box-shadow: 0 0 0 2px rgba(43, 127, 255, 0.2);
                }}
                "#
            }
        }
    }
}
