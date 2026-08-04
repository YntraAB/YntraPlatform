use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct DropdownProps {
    pub label: String,
    #[props(optional)]
    pub icon_url: Option<String>,
    #[props(optional)]
    pub icon_name: Option<&'static str>,
    #[props(optional)]
    pub align_right: Option<bool>,
    pub open: bool,
    pub ontoggle: EventHandler<()>,
    pub children: Element,
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub trigger_class: String,
    #[props(default = String::new())]
    pub style: String,
    #[props(default = String::new())]
    pub trigger_style: String,
}

#[component]
pub fn Dropdown(props: DropdownProps) -> Element {
    let icon_name = props.icon_name;
    let icon_url = props.icon_url.clone();
    let label = props.label.clone();
    let open = props.open;
    let align_right = props.align_right.unwrap_or(false);
    let menu_class = if align_right {
        "dx-dropdown-menu-content dx-dropdown-menu-content-right"
    } else {
        "dx-dropdown-menu-content"
    };
    let ontoggle = props.ontoggle;
    let children = props.children.clone();

    rsx! {
        div {
            class: "dx-dropdown-menu {props.class}",
            style: "width: auto; {props.style}",
            button {
                class: "dx-dropdown-menu-trigger {props.trigger_class}",
                style: "display: flex; justify-content: space-between; align-items: center; width: auto; gap: 0.75rem; box-sizing: border-box; white-space: nowrap; {props.trigger_style}",
                onclick: move |_| ontoggle.call(()),
                div {
                    style: "display: flex; align-items: center; gap: 0.5rem; white-space: nowrap;",
                    if let Some(icon_name) = icon_name {
                        crate::components::LucideIcon {
                            name: icon_name,
                            size: "16",
                            class: "yntra-dropdown-icon shrink-0",
                        }
                    } else if let Some(url) = icon_url.as_ref() {
                        img {
                            src: "{url}",
                            style: "width: 18px; height: auto; border-radius: 2px; margin-right: 0.25rem;",
                        }
                    }
                    span { class: "whitespace-nowrap", "{label}" }
                }
                crate::components::LucideIcon {
                    name: "chevron-down",
                    size: "14",
                    class: format!("opacity-60 transition-transform duration-200 shrink-0 {}", if open { "rotate-180" } else { "" }),
                }
            }
            if open {
                div {
                    class: "bg-transparent",
                    style: "position: fixed; inset: 0; z-index: 999; cursor: default;",
                    onclick: move |_| ontoggle.call(()),
                }
                div {
                    class: "{menu_class}",
                    "data-state": "open",
                    style: "width: 100%; box-sizing: border-box;",
                    {children}
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct DropdownItemProps {
    pub label: String,
    pub onclick: EventHandler<MouseEvent>,
}

#[component]
pub fn DropdownItem(props: DropdownItemProps) -> Element {
    rsx! {
        div {
            class: "dx-dropdown-menu-item",
            onclick: move |evt| props.onclick.call(evt),
            "{props.label}"
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct SelectProps {
    pub value: String,
    pub onchange: EventHandler<String>,
    pub options: Vec<(String, String)>, // (value, label)
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    #[props(default = String::new())]
    pub trigger_class: String,
    #[props(default = String::new())]
    pub trigger_style: String,
    #[props(optional)]
    pub icon_name: Option<&'static str>,
}

#[component]
pub fn Select(props: SelectProps) -> Element {
    let mut open = use_signal(|| false);
    let current_option_label = props
        .options
        .iter()
        .find(|(val, _)| val == &props.value)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| props.value.clone());

    rsx! {
        crate::components::Dropdown {
            label: current_option_label,
            open: *open.read(),
            ontoggle: move |_| {
                let cur = *open.read();
                open.set(!cur);
            },
            class: props.class.clone(),
            style: props.style.clone(),
            trigger_class: props.trigger_class.clone(),
            trigger_style: props.trigger_style.clone(),
            icon_name: props.icon_name,
            for (val, label) in props.options.iter().cloned() {
                {
                    let onchange = props.onchange;
                    rsx! {
                        crate::components::DropdownItem {
                            label: label,
                            onclick: move |_| {
                                onchange.call(val.clone());
                                open.set(false);
                            }
                        }
                    }
                }
            }
        }
    }
}
