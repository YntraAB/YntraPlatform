use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct InputProps {
    #[props(default = String::new())]
    pub placeholder: String,
    pub value: String,
    pub oninput: EventHandler<FormEvent>,
    #[props(default = String::new())]
    pub class: String,
}

#[component]
pub fn Input(props: InputProps) -> Element {
    rsx! {
        input {
            class: "yntra-input {props.class}",
            r#type: "text",
            placeholder: "{props.placeholder}",
            value: "{props.value}",
            oninput: move |evt| props.oninput.call(evt),
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
