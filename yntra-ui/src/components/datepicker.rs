use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct DatePickerProps {
    pub value: String,
    pub onchange: EventHandler<FormEvent>,
    #[props(default = String::new())]
    pub class: String,
}

#[component]
pub fn DatePicker(props: DatePickerProps) -> Element {
    rsx! {
        input {
            class: "yntra-datepicker {props.class}",
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
                "#
            }
        }
    }
}
