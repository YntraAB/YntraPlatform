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
        div { class: "dx-dropdown-menu",
            button {
                class: "dx-dropdown-menu-trigger",
                onclick: move |_| ontoggle.call(()),
                if let Some(icon_name) = icon_name {
                    crate::components::LucideIcon {
                        name: icon_name,
                        size: "16",
                        class: "yntra-dropdown-icon",
                    }
                } else if let Some(url) = icon_url.as_ref() {
                    img {
                        src: "{url}",
                        style: "width: 18px; height: auto; border-radius: 2px; margin-right: 0.25rem;",
                    }
                }
                "{label}"
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
