use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct NativeSelectProps {
    pub value: String,
    #[props(optional)]
    pub onchange: EventHandler<FormEvent>,
    #[props(optional)]
    pub oninput: EventHandler<FormEvent>,
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
    #[props(optional)]
    pub aria_describedby: Option<String>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(default = String::new())]
    pub name: String,
    #[props(optional)]
    pub error: Option<String>,
    #[props(default = false)]
    pub is_invalid: bool,
    pub children: Element,
}

#[component]
pub fn NativeSelect(props: NativeSelectProps) -> Element {
    let is_invalid = props.is_invalid || props.error.is_some();
    let invalid_cls = if is_invalid { "is-invalid" } else { "" };
    rsx! {
        select {
            class: "yntra-input yntra-select {props.class} {invalid_cls}",
            style: "{props.style}",
            value: "{props.value}",
            disabled: props.disabled,
            required: props.required,
            name: if props.name.is_empty() { None } else { Some(props.name.clone()) },
            id: if props.id.is_empty() { None } else { Some(props.id.clone()) },
            aria_label: if props.aria_label.is_empty() { None } else { Some(props.aria_label.clone()) },
            aria_describedby: props.aria_describedby.as_ref().filter(|s| !s.is_empty()).cloned(),
            aria_invalid: if is_invalid { "true" } else { "false" },
            onchange: move |evt| props.onchange.call(evt),
            oninput: move |evt| props.oninput.call(evt),
            onblur: move |evt| props.onblur.call(evt),
            onfocus: move |evt| props.onfocus.call(evt),
            {props.children}
        }
        style {
            r#"
            :where(.yntra-select) {{
                cursor: pointer;
                appearance: none;
                background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='rgba(255,255,255,0.6)' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
                background-repeat: no-repeat;
                background-position: right 0.75rem center;
                background-size: 1rem;
                padding-right: 2.25rem;
            }}
            :where(.yntra-select option) {{
                background-color: #18181b;
                color: var(--text-primary, #ffffff);
            }}
            "#
        }
    }
}

pub type SelectProps = NativeSelectProps;
