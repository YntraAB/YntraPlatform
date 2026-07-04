#[allow(unused_imports)]
use super::NotificationsSettings;
#[allow(unused_imports)]
use super::SchedulerSettings;
#[allow(unused_imports)]
use super::FinanceSettings;
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{update_workspace_modules, BlockItem, Workspace};

#[derive(Props, Clone)]
pub struct BlockSettingsProps {
    pub settings_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub messaging_enabled: bool,
    pub scheduling_enabled: bool,
    pub notes_enabled: bool,
    pub time_enabled: bool,
    pub journals_enabled: bool,
    pub medications_enabled: bool,
    pub directory_enabled: bool,
    pub reporting_enabled: bool,
    pub academics_enabled: bool,
    pub attendance_enabled: bool,
    pub finance_enabled: bool,
    pub library_enabled: bool,
    pub active_user: yntra_core::WorkspaceUser,
    pub account_preferences: Signal<String>,
    pub workspace: Workspace,
    pub locale: String,
}

impl PartialEq for BlockSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn BlockSettings(props: BlockSettingsProps) -> Element {
    let mut db_trigger = props.db_trigger;
    let mut settings_save_status = props.settings_save_status;

    let mut search_term = use_signal(String::new);
    let mut selected_category = use_signal(|| "all".to_string());
    let mut config_block = use_signal(|| Option::<BlockItem>::None);

    let mut available_blocks = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);

    // Fetch blocks dynamically from SQLite database
    let _blocks = use_resource(move || {
        let trigger = *db_trigger.read();
        async move {
            let _ = trigger; // read trigger to react
            if let Ok(list) = yntra_core::get_blocks().await {
                available_blocks.set(list);
            }
            is_loading.set(false);
        }
    });

    let handle_toggle = {
        let locale = props.locale.clone();
        let messaging_enabled = props.messaging_enabled;
        let scheduling_enabled = props.scheduling_enabled;
        let notes_enabled = props.notes_enabled;
        let time_enabled = props.time_enabled;
        let journals_enabled = props.journals_enabled;
        let medications_enabled = props.medications_enabled;
        let directory_enabled = props.directory_enabled;
        let reporting_enabled = props.reporting_enabled;
        let academics_enabled = props.academics_enabled;
        let attendance_enabled = props.attendance_enabled;
        let finance_enabled = props.finance_enabled;
        let library_enabled = props.library_enabled;
        let workspace_id = props.workspace.id.clone();
        
        move |block: BlockItem, next_state: bool| {
            if next_state {
                // Check dependencies
                let deps: Vec<String> = serde_json::from_str(&block.dependencies).unwrap_or_default();
                let is_mod_enabled = |mod_id: &str| -> bool {
                    match mod_id {
                        "messaging" => messaging_enabled,
                        "scheduling" => scheduling_enabled,
                        "notes" => notes_enabled,
                        "time" => time_enabled,
                        "assistance" => journals_enabled || medications_enabled,
                        "journals" => journals_enabled,
                        "medications" => medications_enabled,
                        "directory" => directory_enabled,
                        "reporting" => reporting_enabled,
                        "academics" => academics_enabled,
                        "attendance" => attendance_enabled,
                        "finance" => finance_enabled,
                        "library" => library_enabled,
                        _ => false,
                    }
                };
                let missing: Vec<String> = deps
                    .into_iter()
                    .filter(|dep_id| !is_mod_enabled(dep_id))
                    .collect();

                if !missing.is_empty() {
                    let missing_str = missing.join(", ");
                    settings_save_status.set(format!("error:{}", t("blocks-missing-deps", &locale).replace("{deps}", &missing_str)));
                    return;
                }
            }

            settings_save_status.set("saving".to_string());
            let mut map = serde_json::Map::new();
            for b in available_blocks.read().iter() {
                let active = if b.id == block.id {
                    next_state
                } else {
                    match b.id.as_str() {
                        "messaging" => messaging_enabled,
                        "scheduling" => scheduling_enabled,
                        "notes" => notes_enabled,
                        "time" => time_enabled,
                        "assistance" => journals_enabled || medications_enabled,
                        "journals" => journals_enabled,
                        "medications" => medications_enabled,
                        "directory" => directory_enabled,
                        "reporting" => reporting_enabled,
                        "academics" => academics_enabled,
                        "attendance" => attendance_enabled,
                        "finance" => finance_enabled,
                        "library" => library_enabled,
                        _ => false,
                    }
                };
                map.insert(b.id.clone(), serde_json::Value::Bool(active));
            }

            let new_json = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
            let ws_id = workspace_id.clone();
            let new_json_clone = new_json.clone();
            spawn(async move {
                let _ = update_workspace_modules(ws_id, new_json_clone).await;
            });
            
            let current = *db_trigger.read();
            db_trigger.set(current + 1);
            settings_save_status.set("saved".to_string());
        }
    };

    if *is_loading.read() {
        return rsx! {
            div { class: "grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3",
                for i in 0..6 {
                    div { key: "{i}", class: "h-48 animate-pulse rounded-xl bg-muted/50" }
                }
            }
        };
    }

    // Dynamic category computation
    let mut categories = vec!["all".to_string()];
    for b in available_blocks.read().iter() {
        let cat = b.category.to_lowercase();
        if !categories.contains(&cat) {
            categories.push(cat);
        }
    }

    let filtered_blocks: Vec<BlockItem> = available_blocks
        .read()
        .iter()
        .filter(|b| {
            let search = search_term.read().to_lowercase();
            let matches_search = b.name.to_lowercase().contains(&search)
                || b.description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&search);

            let cat = selected_category.read().clone();
            let matches_category = cat == "all" || b.category.to_lowercase() == cat;

            matches_search && matches_category
        })
        .cloned()
        .collect();

    let config_block_val = config_block.read().clone();

    rsx! {
        div { class: "space-y-6",
            // Header
            div {
                h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0",
                    "{t(\"settings-blocks-title\", &props.locale)}"
                }
                p { class: "text-sm text-muted-foreground m-0 mt-1",
                    "{t(\"settings-blocks-desc\", &props.locale)}"
                }
            }

            // Filters
            div { class: "flex flex-col gap-4 sm:flex-row sm:items-center",
                div { class: "relative w-full sm:w-80",
                    components::LucideIcon {
                        name: "search",
                        class: "absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground",
                    }
                    components::Input {
                        placeholder: t("settings-blocks-search-placeholder", &props.locale),
                        class: "pl-9",
                        value: "{search_term}",
                        oninput: move |e: FormEvent| search_term.set(e.value())
                    }
                }
                div { class: "flex flex-wrap gap-2",
                    for cat in categories.iter() {
                        {
                            let is_active = *selected_category.read() == *cat;
                            let cat_clone = cat.clone();
                            let cat_label = if cat == "all" {
                                t("settings-blocks-filter-all", &props.locale)
                            } else {
                                t(&format!("blocks-categories-{}", cat), &props.locale)
                            };
                            
                            rsx! {
                                components::Button {
                                    key: "{cat}",
                                    variant: if is_active { components::ButtonVariant::Primary } else { components::ButtonVariant::Secondary },
                                    class: "h-8 text-xs font-medium px-4 py-0 rounded-lg",
                                    onclick: move |_| selected_category.set(cat_clone.clone()),
                                    span { "{cat_label}" }
                                }
                            }
                        }
                    }
                }
            }

            // Blocks Grid
            if filtered_blocks.is_empty() {
                div { class: "flex h-40 flex-col items-center justify-center rounded-xl border border-dashed border-border text-muted-foreground",
                    components::LucideIcon { name: "layout-grid", class: "mb-2 h-8 w-8 opacity-20" }
                    p { class: "text-sm m-0", "{t(\"settings-blocks-no-results\", &props.locale)}" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3",
                    for block in filtered_blocks.iter() {
                        {
                            let block_item = block.clone();
                            let block_id = block.id.clone();
                            let block_icon = block.icon.clone();
                            
                            let block_name_key = format!("settings-blocks-{}-name", block_id);
                            let block_name_translated = t(&block_name_key, &props.locale);
                            
                            let block_desc_key = format!("settings-blocks-{}-desc", block_id);
                            let block_desc_translated = t(&block_desc_key, &props.locale);
                            
                            let is_enabled = match block.id.as_str() {
                                "messaging" => props.messaging_enabled,
                                "scheduling" => props.scheduling_enabled,
                                "notes" => props.notes_enabled,
                                "time" => props.time_enabled,
                                "assistance" => props.journals_enabled || props.medications_enabled,
                                "journals" => props.journals_enabled,
                                "medications" => props.medications_enabled,
                                "directory" => props.directory_enabled,
                                "reporting" => props.reporting_enabled,
                                "academics" => props.academics_enabled,
                                "attendance" => props.attendance_enabled,
                                "finance" => props.finance_enabled,
                                "library" => props.library_enabled,
                                _ => false,
                            };
                            
                            let is_configurable = block.id == "scheduling" || block.id == "messaging" || block.id == "finance";
                            
                            rsx! {
                                components::Card {
                                    key: "{block.id}",
                                    class: format!(
                                        "group relative cursor-pointer overflow-hidden border-border/50 transition-all duration-300 hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5 {}",
                                        if is_enabled { "border-primary/30 bg-primary/[0.02] ring-1 ring-primary/10" } else { "" }
                                    ),
                                    onclick: {
                                        let handle_toggle = handle_toggle.clone();
                                        let block_item = block_item.clone();
                                        move |_| {
                                            let mut handle_toggle = handle_toggle.clone();
                                            handle_toggle(block_item.clone(), !is_enabled);
                                        }
                                    },
                                    components::CardHeader { class: "p-4 pb-2",
                                        div { class: "flex items-center justify-between",
                                            div {
                                                class: format!(
                                                    "rounded-lg p-2 transition-all duration-300 group-hover:scale-105 {}",
                                                    if is_enabled { "bg-primary text-primary-foreground shadow-md shadow-primary/20" } else { "bg-muted text-muted-foreground" }
                                                ),
                                                components::LucideIcon { name: block_icon, class: "h-4 w-4" }
                                            }
                                            div {
                                                class: "flex items-center gap-2",
                                                onclick: move |e| e.stop_propagation(),
                                                if is_configurable && is_enabled {
                                                    button {
                                                        class: "h-7 w-7 rounded-full bg-transparent border-0 flex items-center justify-center hover:bg-muted text-muted-foreground cursor-pointer transition-colors",
                                                        onclick: {
                                                            let block_item = block_item.clone();
                                                            move |_| {
                                                                config_block.set(Some(block_item.clone()));
                                                            }
                                                        },
                                                        components::LucideIcon { name: "settings-2", class: "h-3.5 w-3.5" }
                                                    }
                                                }
                                                components::Switch {
                                                    checked: is_enabled,
                                                    onchange: {
                                                        let handle_toggle = handle_toggle.clone();
                                                        let block_item = block_item.clone();
                                                        move |val| {
                                                            let mut handle_toggle = handle_toggle.clone();
                                                            handle_toggle(block_item.clone(), val);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "mt-3 flex items-center justify-between gap-2",
                                            components::CardTitle { class: "text-sm font-bold tracking-tight",
                                                "{block_name_translated}"
                                            }
                                            if is_enabled {
                                                div { class: "h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse" }
                                            }
                                        }
                                    }
                                    components::CardContent { class: "p-4 pt-0",
                                        components::CardDescription { class: "line-clamp-2 text-[11px] leading-relaxed text-muted-foreground/80",
                                            "{block_desc_translated}"
                                        }
                                    }
                                    if is_enabled {
                                        div { class: "absolute -right-10 -top-10 h-20 w-20 rotate-45 bg-primary/5 transition-transform duration-500 group-hover:scale-110" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Dialog Modal for active block configuration
            if let Some(cfg_block) = config_block_val {
                {
                    let cfg_block_id = cfg_block.id.clone();
                    let cfg_block_icon = cfg_block.icon.clone();
                    
                    let cfg_name_key = format!("settings-blocks-{}-name", cfg_block_id);
                    let cfg_name_translated = t(&cfg_name_key, &props.locale);
                    
                    let cfg_desc_key = format!("settings-blocks-{}-desc", cfg_block_id);
                    let cfg_desc_translated = t(&cfg_desc_key, &props.locale);
                    
                    rsx! {
                        components::Dialog {
                            open: true,
                            title: cfg_name_translated.clone(),
                            onclose: move |_| config_block.set(None),
                            max_width: if cfg_block_id == "scheduling" { Some("1000px".to_string()) } else { Some("640px".to_string()) },
                            div { 
                                class: format!(
                                    "flex flex-col gap-5 w-full text-sm max-h-[75vh] overflow-y-auto pr-2 {}",
                                    if cfg_block_id == "scheduling" { "max-w-5xl" } else { "max-w-2xl" }
                                ),
                                div { class: "flex items-center gap-3 border-b border-border/50 pb-4",
                                    div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                        components::LucideIcon { name: cfg_block_icon, class: "h-5 w-5" }
                                    }
                                    div {
                                        h4 { class: "m-0 font-bold text-foreground text-base",
                                            "{cfg_name_translated}"
                                        }
                                        p { class: "m-0 text-xs text-muted-foreground/70 mt-0.5 leading-relaxed",
                                            "{cfg_desc_translated}"
                                        }
                                    }
                                }

                                div { class: "mt-2",
                                    if cfg_block_id == "scheduling" {
                                        SchedulerSettings {
                                            settings_save_status: props.settings_save_status,
                                            db_trigger: props.db_trigger,
                                            workspace: props.workspace.clone(),
                                            locale: props.locale.clone(),
                                        }
                                    } else if cfg_block_id == "finance" {
                                        FinanceSettings {
                                            settings_save_status: props.settings_save_status,
                                            db_trigger: props.db_trigger,
                                            workspace: props.workspace.clone(),
                                            locale: props.locale.clone(),
                                        }
                                    } else if cfg_block_id == "messaging" {
                                        NotificationsSettings {
                                            settings_save_status: props.settings_save_status,
                                            active_user: props.active_user.clone(),
                                            account_preferences: props.account_preferences,
                                            db_trigger: props.db_trigger,
                                            locale: props.locale.clone(),
                                        }
                                    } else {
                                        div { class: "flex flex-col items-center justify-center py-12 text-center",
                                            div { class: "mb-4 rounded-full bg-muted p-4",
                                                components::LucideIcon { name: "settings-2", class: "h-8 w-8 text-muted-foreground opacity-20" }
                                            }
                                            h3 { class: "text-lg font-medium m-0", "{t(\"settings-blocks-config-placeholder-title\", &props.locale)}" }
                                            p { class: "max-w-xs text-sm text-muted-foreground m-0 mt-1.5",
                                                "{t(\"settings-blocks-config-placeholder-desc\", &props.locale)}"
                                            }
                                        }
                                    }
                                }

                                div { class: "flex justify-end border-t border-border/50 pt-4 mt-2",
                                    components::Button {
                                        onclick: move |_| config_block.set(None),
                                        span { "Done" }
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
