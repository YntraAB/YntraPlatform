use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct NumberInputProps {
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
    #[props(default = String::new())]
    pub aria_label: String,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(default = String::new())]
    pub name: String,
    #[props(default = String::new())]
    pub step: String,
    #[props(default = String::new())]
    pub min: String,
    #[props(default = String::new())]
    pub max: String,
}

#[component]
pub fn NumberInput(props: NumberInputProps) -> Element {
    rsx! {
        input {
            class: "yntra-input yntra-number-input {props.class}",
            style: "{props.style}",
            r#type: "number",
            placeholder: "{props.placeholder}",
            value: "{props.value}",
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            name: if props.name.is_empty() { None } else { Some(props.name.clone()) },
            id: if props.id.is_empty() { None } else { Some(props.id.clone()) },
            aria_label: if props.aria_label.is_empty() { None } else { Some(props.aria_label.clone()) },
            step: if props.step.is_empty() { None } else { Some(props.step.clone()) },
            min: if props.min.is_empty() { None } else { Some(props.min.clone()) },
            max: if props.max.is_empty() { None } else { Some(props.max.clone()) },
            oninput: move |evt| props.oninput.call(evt),
            onchange: move |evt| props.onchange.call(evt),
            onkeydown: move |evt| props.onkeydown.call(evt),
            onblur: move |evt| props.onblur.call(evt),
            onfocus: move |evt| props.onfocus.call(evt),
        }
    }
}
