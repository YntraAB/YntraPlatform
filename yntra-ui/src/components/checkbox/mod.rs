use dioxus::prelude::*;
use dioxus_icons::lucide::Check;
use dioxus_primitives::checkbox::{self};

#[css_module("/src/components/checkbox/style.css")]
struct Styles;

#[derive(Props, Clone, PartialEq)]
pub struct CheckboxProps {
    pub checked: bool,
    pub onchange: EventHandler<bool>,

    #[props(default = String::new())]
    pub label: String,

    #[props(default)]
    pub disabled: bool,
}

#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    let mut checked_state = use_signal(|| {
        let state = if props.checked {
            dioxus_primitives::checkbox::CheckboxState::Checked
        } else {
            dioxus_primitives::checkbox::CheckboxState::Unchecked
        };
        Some(state)
    });
    use_effect(move || {
        let state = if props.checked {
            dioxus_primitives::checkbox::CheckboxState::Checked
        } else {
            dioxus_primitives::checkbox::CheckboxState::Unchecked
        };
        checked_state.set(Some(state));
    });

    let onchange = props.onchange;
    let on_checked_change =
        Callback::new(move |val: dioxus_primitives::checkbox::CheckboxState| {
            let is_checked = matches!(val, dioxus_primitives::checkbox::CheckboxState::Checked);
            onchange.call(is_checked);
        });

    rsx! {
        div {
            class: "yntra-checkbox-wrapper",
            checkbox::Checkbox {
                class: Styles::dx_checkbox,
                checked: ReadSignal::new(checked_state),
                disabled: ReadSignal::new(Signal::new(props.disabled)),
                on_checked_change: on_checked_change,
                checkbox::CheckboxIndicator { class: Styles::dx_checkbox_indicator,
                    Check { size: "1rem" }
                }
            }
            if !props.label.is_empty() {
                span { class: "yntra-checkbox-label",
                    style: "color: var(--secondary-color-4); font-size: 0.95rem; margin-left: 0.5rem; cursor: pointer; user-select: none;",
                    onclick: move |_| {
                        if !props.disabled {
                            onchange.call(!props.checked);
                        }
                    },
                    "{props.label}"
                }
            }
        }
    }
}
