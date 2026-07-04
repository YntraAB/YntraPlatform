use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub children: Element,
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    rsx! {
        aside { class: "flex h-full w-64 flex-col border-r border-border bg-sidebar",
            {props.children}
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct SidebarItemProps {
    pub active: bool,
    pub label: String,
    pub onclick: EventHandler<MouseEvent>,
}

#[component]
pub fn SidebarItem(props: SidebarItemProps) -> Element {
    let active_class = if props.active { "bg-muted text-foreground" } else { "text-muted-foreground hover:bg-muted hover:text-foreground" };
    rsx! {
        div {
            class: "flex cursor-pointer items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-all duration-150 mb-1 {active_class}",
            onclick: move |evt| props.onclick.call(evt),
            "{props.label}"
        }
    }
}
