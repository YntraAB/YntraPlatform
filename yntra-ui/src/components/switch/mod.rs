use dioxus::prelude::*;

#[css_module("/src/components/switch/style.css")]
struct Styles;

#[derive(Props, Clone, PartialEq)]
pub struct SwitchProps {
    pub checked: bool,
    pub onchange: EventHandler<bool>,

    #[props(default)]
    pub disabled: bool,
}

#[component]
pub fn Switch(props: SwitchProps) -> Element {
    let checked = props.checked;
    let disabled = props.disabled;
    let onchange = props.onchange;

    rsx! {
        button {
            role: "switch",
            aria_checked: "{checked}",
            disabled: disabled,
            class: Styles::dx_switch,
            "data-state": if checked { "checked" } else { "unchecked" },
            "data-disabled": if disabled { "true" } else { "false" },
            onclick: move |e| {
                e.stop_propagation();
                if !disabled {
                    onchange.call(!checked);
                }
            },
            span {
                class: Styles::dx_switch_thumb,
            }
        }
    }
}
