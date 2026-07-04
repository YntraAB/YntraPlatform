use dioxus::prelude::*;
use dioxus_primitives::switch::{self};

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
    let mut checked = use_signal(|| Some(props.checked));
    // Keep internal signal in sync with props changes
    use_effect(move || {
        checked.set(Some(props.checked));
    });

    let onchange = props.onchange;
    let on_checked_change = Callback::new(move |val: bool| {
        onchange.call(val);
    });

    rsx! {
        switch::Switch {
            class: Styles::dx_switch,
            checked: ReadSignal::new(checked),
            disabled: ReadSignal::new(Signal::new(props.disabled)),
            on_checked_change: on_checked_change,
            switch::SwitchThumb { class: Styles::dx_switch_thumb }
        }
    }
}
