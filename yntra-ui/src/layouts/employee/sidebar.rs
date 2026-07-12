use crate::components;
use crate::locales::t;
use crate::blocks;
use dioxus::prelude::*;
use yntra_core::Workspace;

#[derive(Props, Clone)]
pub struct LayoutSidebarProps {
    pub workspace: Workspace,
    pub active_user_role: String,
    pub unread_messages_count: usize,
    pub active_section: Signal<String>,
    pub globalsearch_open: Signal<bool>,
    pub selected_note_team_id: Signal<String>,
    pub time_group_expanded: Signal<bool>,
    pub filter_categories: Signal<Vec<String>>,
    pub auth_region: Signal<String>,
    pub messaging_enabled: bool,
    pub scheduling_enabled: bool,
    pub notes_enabled: bool,
    pub time_enabled: bool,
    pub directory_enabled: bool,
    pub reporting_enabled: bool,
    pub jobs_enabled: bool,
    pub todos_enabled: bool,
}

impl PartialEq for LayoutSidebarProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn LayoutSidebar(props: LayoutSidebarProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let workspace = props.workspace.clone();
    let teams = state.teams.read().clone().unwrap_or_default();
    let current_role = props.active_user_role.clone();
    let unread_messages_count = props.unread_messages_count;

    let mut active_section = props.active_section;
    let mut globalsearch_open = props.globalsearch_open;
    let mut selected_note_team_id = props.selected_note_team_id;
    let mut time_group_expanded = props.time_group_expanded;
    let auth_region = props.auth_region;
    let mut dropdown_open = use_signal(|| false);

    let is_client = current_role == "client";
    let is_admin = current_role == "admin" || current_role == "platform_admin";

    let chevron_icon = if *time_group_expanded.read() { "chevron-down" } else { "chevron-right" };

    let get_nav_item_class = |sec: &str| {
        let active = *active_section.read() == sec;
        let theme_class = if active { "bg-muted text-foreground active" } else { "text-muted-foreground hover:bg-muted hover:text-foreground" };
        format!("sidebar-item relative flex cursor-pointer items-center justify-between px-3 py-2 rounded-md transition-all duration-150 text-sm font-medium mb-1 {}", theme_class)
    };

    let db_trigger = state.db_trigger;
    let db_trig_val = *db_trigger.read();
    let blocks_res = use_resource(move || {
        let _ = db_trig_val;
        async move {
            yntra_core::get_blocks().await.unwrap_or_default()
        }
    });
    let db_blocks = blocks_res.read().clone().unwrap_or_default();
    let modules_active_val: serde_json::Value =
        serde_json::from_str(&workspace.modules_active).unwrap_or_default();

    rsx! {
        aside { class: "flex h-full w-64 flex-col border-r border-border bg-sidebar",
            // Header / Logo Area
            div { class: "border-b border-border p-4",
                div { class: "flex items-center gap-3",
                    if let Some(ref logo) = workspace.logo_url {
                        img {
                            src: "{logo}",
                            class: "h-8 w-8 rounded-lg border border-border/50 object-contain shadow-sm",
                        }
                    } else {
                        svg {
                            width: "28",
                            height: "28",
                            view_box: "0 0 48 48",
                            fill: "none",
                            class: "-rotate-12 transform",
                            path {
                                d: "M28 4L12 24H22L18 44L36 20H24L28 4Z",
                                fill: "#8B5CF6",
                                stroke: "#8B5CF6",
                                stroke_width: "2",
                                stroke_linejoin: "round",
                            }
                        }
                    }
                    div { class: "min-w-0 flex-1",
                        h2 { class: "truncate text-sm font-bold leading-tight text-foreground", "{workspace.name}" }
                        p { class: "text-xs text-muted-foreground", "EE24" }
                    }
                }
            }
            
            // Search Trigger in Sidebar under Logo
            div { class: "px-3 py-3",
                div {
                    onclick: move |_| globalsearch_open.set(true),
                    class: "flex cursor-pointer items-center gap-2 rounded-md border border-border/50 bg-background/50 px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-muted",
                    components::LucideIcon { name: "search", size: "14" }
                    span { class: "flex-1", "{t(\"common-search\", &auth_region.read())}..." }
                    kbd { class: "hidden rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium sm:inline-block", "⌘K" }
                }
            }
            
            // Team Switcher (Admins only)
            if is_admin {
                div { class: "mb-5 px-4 flex flex-col gap-1.5",
                    div { class: "flex items-center gap-1.5 pl-1",
                        components::LucideIcon { name: "directory", class: "h-3.5 w-3.5 text-muted-foreground" }
                        span { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-active-schedule\", &auth_region.read())}"
                        }
                    }
                    {
                        let dropdown_label = {
                            let selected_id = selected_note_team_id.read();
                            if selected_id.is_empty() {
                                t("scheduler-all-teams", &auth_region.read())
                            } else {
                                teams.iter()
                                    .find(|t| t.id == *selected_id)
                                    .map(|t| t.name.clone())
                                    .unwrap_or_else(|| t("scheduler-all-teams", &auth_region.read()))
                            }
                        };
                        
                        rsx! {
                            components::Dropdown {
                                label: dropdown_label,
                                open: *dropdown_open.read(),
                                ontoggle: move |_| {
                                    let current = *dropdown_open.read();
                                    dropdown_open.set(!current);
                                },
                                components::DropdownItem {
                                    label: t("scheduler-all-teams", &auth_region.read()),
                                    onclick: move |_| {
                                        selected_note_team_id.set(String::new());
                                        dropdown_open.set(false);
                                    }
                                }
                                for t in teams.iter() {
                                    {
                                        let t_id = t.id.clone();
                                        let t_name = t.name.clone();
                                        rsx! {
                                            components::DropdownItem {
                                                label: "{t_name}",
                                                onclick: move |_| {
                                                    selected_note_team_id.set(t_id.clone());
                                                    dropdown_open.set(false);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Navigation Menu List
            div { class: "flex flex-col gap-1 flex-1 overflow-y-auto px-3 py-2",
                {
                    let is_module_enabled = |block_id: &str| -> bool {
                        match block_id {
                            "dashboard" => true,
                            "messaging" => props.messaging_enabled,
                            "scheduling" => props.scheduling_enabled,
                            "notes" => props.notes_enabled,
                            "client_portal" => false,
                            "directory" => props.directory_enabled,
                            "reporting" => props.reporting_enabled,
                            "jobs" => props.jobs_enabled,
                            "todos" => props.todos_enabled,
                            "p2p_playground" => true,
                            _ => false,
                        }
                    };

                    let is_role_allowed = |allowed_roles: Option<&[&str]>| -> bool {
                        if let Some(roles) = allowed_roles {
                            roles.contains(&current_role.as_str())
                        } else {
                            true
                        }
                    };

                    let is_child_enabled = |required_id: Option<&str>| -> bool {
                        match required_id {
                            Some("time") => props.time_enabled,
                            Some(id) => is_module_enabled(id),
                            None => true,
                        }
                    };

                    let static_block_ids: std::collections::HashSet<&str> = blocks::BLOCK_REGISTRY.iter().map(|b| b.id).collect();

                    rsx! {
                        for block in blocks::BLOCK_REGISTRY.iter() {
                            if block.id == "dashboard" || is_module_enabled(block.id) {
                                for item in block.navigation.iter() {
                                    if is_role_allowed(item.allowed_roles) {
                                        {
                                            let item_id = item.id;
                                            let item_path = item.path;
                                            let item_icon = item.icon;
                                            let item_badge = item.badge_key;
                                            let has_children = item.children.is_some();
                                            let display_label = {
                                                let block_settings_val: serde_json::Value =
                                                    serde_json::from_str(&workspace.block_settings).unwrap_or_default();
                                                let override_name = block_settings_val.get(item_id)
                                                    .and_then(|b| b.get("display_name"))
                                                    .and_then(|n| n.as_str())
                                                    .map(|s| s.to_string());
                                                
                                                if let Some(name) = override_name {
                                                    name
                                                } else if item.id == "messaging" && is_client {
                                                    "Care Chat".to_string()
                                                } else {
                                                    t(item.label_key, &auth_region.read())
                                                }
                                            };
                                            
                                            rsx! {
                                                div {
                                                    key: "{item_id}",
                                                    class: if has_children {
                                                        "flex cursor-pointer items-center justify-between px-3 py-2 rounded-md transition-all duration-150 text-sm font-medium text-muted-foreground hover:bg-muted hover:text-foreground mb-1".to_string()
                                                    } else {
                                                        get_nav_item_class(item_id)
                                                    },
                                                    onclick: move |_| {
                                                        if has_children {
                                                            let cur = *time_group_expanded.read();
                                                            time_group_expanded.set(!cur);
                                                        } else {
                                                            active_section.set(item_path.to_string());
                                                        }
                                                    },
                                                    "data-testid": "sidebar-item-{item_id}",
                                                    div { class: "flex items-center gap-3",
                                                        components::LucideIcon { name: item_icon, size: "16" }
                                                        span { "{display_label}" }
                                                    }
                                                    if has_children {
                                                        components::LucideIcon {
                                                            name: chevron_icon,
                                                            size: "14"
                                                        }
                                                    }
                                                    if let Some(badge_key) = item_badge {
                                                        if badge_key == "unread_messages" && unread_messages_count > 0 {
                                                            span { class: "h-5 min-w-[20px] justify-center px-1.5 text-[10px] font-bold bg-primary text-primary-foreground rounded-full flex items-center justify-center", "{unread_messages_count}" }
                                                        }
                                                    }
                                                }
                                                if has_children && *time_group_expanded.read() {
                                                    if let Some(children) = item.children {
                                                        for child in children.iter() {
                                                            if is_child_enabled(child.required_block_id) {
                                                                {
                                                                    let child_id = child.id;
                                                                    let child_path = child.path;
                                                                    let child_icon = child.icon;
                                                                    let child_label = t(child.label_key, &auth_region.read());
                                                                    rsx! {
                                                                        div {
                                                                            key: "{child_id}",
                                                                            class: "{get_nav_item_class(child_id)}",
                                                                            onclick: move |_| active_section.set(child_path.to_string()),
                                                                            "data-testid": "sidebar-item-{child_id}",
                                                                            style: "padding-left: 2rem;",
                                                                            div { class: "flex items-center gap-3",
                                                                                components::LucideIcon { name: child_icon, size: "14" }
                                                                                span { "{child_label}" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                             }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Custom Dynamic Sidebar Items
                        for b in db_blocks.iter() {
                            if !static_block_ids.contains(b.id.as_str()) && modules_active_val.get(&b.id).and_then(|v| v.as_bool()).unwrap_or(false) {
                                {
                                    let b_id = b.id.clone();
                                    let b_name = b.name.clone();
                                    let b_icon = b.icon.clone();
                                    let active = *active_section.read() == b_id;
                                    let theme_class = if active { "bg-muted text-foreground active" } else { "text-muted-foreground hover:bg-muted hover:text-foreground" };
                                    let b_id_click = b_id.clone();
                                    rsx! {
                                        div {
                                            key: "{b_id}",
                                            class: "sidebar-item relative flex cursor-pointer items-center justify-between px-3 py-2 rounded-md transition-all duration-150 text-sm font-medium mb-1 {theme_class}",
                                            onclick: move |_| active_section.set(b_id_click.clone()),
                                            "data-testid": "sidebar-item-{b_id}",
                                            div { class: "flex items-center gap-3",
                                                components::LucideIcon { name: b_icon, size: "16" }
                                                span { "{b_name}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            

    }
}
}
