#[allow(unused_imports)]
use super::FinanceSettings;
#[allow(unused_imports)]
use super::NotificationsSettings;
#[allow(unused_imports)]
use super::SchedulerSettings;
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{BlockItem, Workspace, update_workspace_modules};

fn get_module_default_state(block_id: &str) -> bool {
    match block_id {
        "messaging" | "scheduling" | "notes" | "time" | "directory" | "reporting" | "jobs"
        | "todos" | "dispatch" | "live_map" | "fleet" | "rut_exports" | "booking_widget" => true,
        _ => false,
    }
}

#[derive(Props, Clone)]
pub struct BlockSettingsProps {
    pub settings_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub messaging_enabled: bool,
    pub scheduling_enabled: bool,
    pub notes_enabled: bool,
    pub time_enabled: bool,
    pub directory_enabled: bool,
    pub reporting_enabled: bool,
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
    let requester_id = props.active_user.id.clone();
    let _blocks = use_resource(move || {
        let trigger = *db_trigger.read();
        let r_id = requester_id.clone();
        async move {
            let _ = trigger; // read trigger to react
            if let Ok(list) = yntra_core::get_blocks(r_id).await {
                available_blocks.set(list);
            }
            is_loading.set(false);
        }
    });

    let handle_toggle = {
        let locale = props.locale.clone();
        let user_id = props.active_user.id.clone();
        let workspace = props.workspace.clone();

        move |block: BlockItem, next_state: bool| {
            println!(
                "handle_toggle called: block_id={}, next_state={}, user_id={}",
                block.id, next_state, user_id
            );
            let modules_active_val: serde_json::Value =
                serde_json::from_str(&workspace.modules_active).unwrap_or_default();

            if next_state {
                // Check dependencies
                let deps: Vec<String> =
                    serde_json::from_str(&block.dependencies).unwrap_or_default();
                let is_mod_enabled = |mod_id: &str| -> bool {
                    modules_active_val
                        .get(mod_id)
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                };

                let missing: Vec<String> = deps
                    .into_iter()
                    .filter(|dep_id| !is_mod_enabled(dep_id))
                    .collect();

                if !missing.is_empty() {
                    let missing_str = missing.join(", ");
                    settings_save_status.set(format!(
                        "error:{}",
                        t("blocks-missing-deps", &locale).replace("{deps}", &missing_str)
                    ));
                    return;
                }
            }

            settings_save_status.set("saving".to_string());
            let mut map = serde_json::Map::new();
            for b in available_blocks.read().iter() {
                let active = if b.id == block.id {
                    next_state
                } else {
                    modules_active_val
                        .get(&b.id)
                        .and_then(|v| v.as_bool())
                        .unwrap_or_else(|| get_module_default_state(&b.id))
                };
                map.insert(b.id.clone(), serde_json::Value::Bool(active));
            }

            let new_json =
                serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
            let ws_id = workspace.id.clone();
            let new_json_clone = new_json.clone();
            let requester_uid = user_id.clone();
            spawn(async move {
                match update_workspace_modules(requester_uid, ws_id, new_json_clone).await {
                    Ok(_) => {
                        let current = *db_trigger.read();
                        db_trigger.set(current + 1);
                        settings_save_status.set("saved".to_string());
                    }
                    Err(e) => {
                        settings_save_status.set(format!("error:{}", e));
                    }
                }
            });
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
        if b.id == "notes" || b.id == "reporting" || b.id == "todos" {
            continue;
        }
        let cat = b.category.to_lowercase();
        if !categories.contains(&cat) {
            categories.push(cat);
        }
    }

    let filtered_blocks: Vec<BlockItem> = available_blocks
        .read()
        .iter()
        .filter(|b| {
            if b.id == "notes" || b.id == "reporting" || b.id == "todos" {
                return false;
            }
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
                        style: "padding-left: 2.25rem;",
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

                            let static_block_ids = vec![
                                "messaging", "scheduling", "notes", "time",
                                "directory", "reporting", "jobs", "todos"
                            ];
                            let is_static = static_block_ids.contains(&block_id.as_str());

                            let block_name_translated = if is_static {
                                let block_name_key = format!("settings-blocks-{}-name", block_id);
                                t(&block_name_key, &props.locale)
                            } else {
                                block.name.clone()
                            };

                            let block_desc_translated = if is_static {
                                let block_desc_key = format!("settings-blocks-{}-desc", block_id);
                                t(&block_desc_key, &props.locale)
                            } else {
                                block.description.clone().unwrap_or_default()
                            };

                            let modules_active_val: serde_json::Value =
                                serde_json::from_str(&props.workspace.modules_active).unwrap_or_default();

                            let is_enabled = if block_id == "assistance" {
                                false
                            } else {
                                modules_active_val.get(&block_id)
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or_else(|| get_module_default_state(&block_id))
                            };

                            let is_configurable = block.id == "scheduling"
                                || block.id == "messaging"
                                || block.id == "finance"
                                || block.id == "jobs"
                                || block.id == "todos"
                                || block.id == "notes"
                                || block.id == "reporting";

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

                    let static_block_ids = vec![
                        "messaging", "scheduling", "notes", "time",
                        "directory", "reporting", "jobs", "todos"
                    ];
                    let is_static = static_block_ids.contains(&cfg_block_id.as_str());

                    let cfg_name_translated = if is_static {
                        let cfg_name_key = format!("settings-blocks-{}-name", cfg_block_id);
                        t(&cfg_name_key, &props.locale)
                    } else {
                        cfg_block.name.clone()
                    };

                    let cfg_desc_translated = if is_static {
                        let cfg_desc_key = format!("settings-blocks-{}-desc", cfg_block_id);
                        t(&cfg_desc_key, &props.locale)
                    } else {
                        cfg_block.description.clone().unwrap_or_default()
                    };

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
                                    } else if cfg_block_id == "jobs" {
                                        JobsSettings {
                                            settings_save_status: props.settings_save_status,
                                            db_trigger: props.db_trigger,
                                            workspace: props.workspace.clone(),
                                            locale: props.locale.clone(),
                                        }
                                    } else if cfg_block_id == "todos" || cfg_block_id == "notes" || cfg_block_id == "reporting" {
                                        BlockCustomUiSettings {
                                            block_id: cfg_block_id.clone(),
                                            settings_save_status: props.settings_save_status,
                                            db_trigger: props.db_trigger,
                                            workspace: props.workspace.clone(),
                                            locale: props.locale.clone(),
                                        }
                                    } else {
                                        div { class: "flex flex-col items-center justify-center py-12 text-center",
                                            div { class: "mb-4 rounded-full bg-muted p-4",
                                                components::LucideIcon { name: "settings-2", class: "h-8 w-8 text-muted-foreground opacity-20" }
                                            }
                                            h3 { class: "text-lg font-medium m-0", 
                                                match props.locale.as_str() {
                                                    "sv" => "Dynamisk databas & formulär",
                                                    "no" => "Dynamisk database & skjema",
                                                    "da" => "Dynamisk database & formular",
                                                    "fi" => "Dynaaminen tietokanta & lomake",
                                                    _ => "Dynamic Database & Forms"
                                                }
                                            }
                                            p { class: "max-w-md text-sm text-muted-foreground m-0 mt-2 leading-relaxed px-4",
                                                match props.locale.as_str() {
                                                    "sv" => "Denna modul drivs av Yntras dynamiska modulmotor. Den använder automatiskt genererade tabellvyer och databasformulär baserade på den definierade schemakonfigurationen.",
                                                    "no" => "Denne modulen er drevet av Yntras dynamiske modulmotor. Den bruker automatisk genererte tabellvisninger og databasskjemaer basert på den definerte skjema-konfigurasjonen.",
                                                    "da" => "Dette modul er drevet af Yntras dynamiske modulmotor. Det bruger automatisk genererede tabelvisninger og databaseformularer baseret på den definerede skemakonfiguration.",
                                                    "fi" => "Tämä moduuli toimii Yntran dynaamisen moduulimoottorin avulla. Se käyttää automaattisesti luotuja taulukkonäkymiä ja tietokantalomakkeita määritetyn skeeman perusteella.",
                                                    _ => "This module is powered by Yntra's dynamic modular engine. It uses automatically generated table views and database forms based on its defined fields schema."
                                                }
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

#[derive(Props, Clone)]
struct BlockCustomUiSettingsProps {
    block_id: String,
    settings_save_status: Signal<String>,
    db_trigger: Signal<u32>,
    workspace: Workspace,
    locale: String,
}

impl PartialEq for BlockCustomUiSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
fn BlockCustomUiSettings(props: BlockCustomUiSettingsProps) -> Element {
    let block_id = props.block_id.clone();
    let mut db_trigger = props.db_trigger;
    let mut settings_save_status = props.settings_save_status;

    let block_settings_val: serde_json::Value =
        serde_json::from_str(&props.workspace.block_settings).unwrap_or_default();

    let use_custom_ui = block_settings_val
        .get(&block_id)
        .and_then(|b| b.get("use_custom_ui"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    rsx! {
        div { class: "space-y-4 py-2",
            div { class: "flex items-center justify-between gap-4 p-4 rounded-xl border border-border/60 bg-muted/20",
                div { class: "flex-1",
                    h5 { class: "text-sm font-bold text-foreground m-0", "Use Specialized App Interface" }
                    p { class: "text-xs text-muted-foreground m-0 mt-1 leading-relaxed",
                        "When enabled, the app renders a tailored, high-fidelity experience optimized for this module. When disabled, it uses generic forms and database schemas defined by the administrator."
                    }
                }
                components::Switch {
                    checked: use_custom_ui,
                    onchange: {
                        let block_id = block_id.clone();
                        let workspace_id = props.workspace.id.clone();
                        let user_id = use_context::<crate::state::AppState>().active_user_id.read().clone();
                        let ws_block_settings_raw = props.workspace.block_settings.clone();

                        move |val| {
                            settings_save_status.set("saving".to_string());
                            let mut settings_map: serde_json::Value = serde_json::from_str(&ws_block_settings_raw).unwrap_or_default();

                            let mut block_map = settings_map.get(&block_id)
                                .and_then(|b| b.as_object())
                                .cloned()
                                .unwrap_or_default();

                            block_map.insert("use_custom_ui".to_string(), serde_json::Value::Bool(val));
                            if let Some(obj) = settings_map.as_object_mut() {
                                obj.insert(block_id.clone(), serde_json::Value::Object(block_map));
                            }

                            let settings_str = serde_json::to_string(&settings_map).unwrap_or_default();
                            let ws_id = workspace_id.clone();
                            let requester_uid = user_id.clone();
                            spawn(async move {
                                match yntra_core::update_workspace_block_settings(requester_uid, ws_id, settings_str).await {
                                    Ok(_) => {
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                        settings_save_status.set("saved".to_string());
                                    }
                                    Err(e) => {
                                        settings_save_status.set(format!("error:{}", e));
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
struct JobsSettingsProps {
    settings_save_status: Signal<String>,
    db_trigger: Signal<u32>,
    workspace: Workspace,
    locale: String,
}

impl PartialEq for JobsSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
fn JobsSettings(props: JobsSettingsProps) -> Element {
    let mut db_trigger = props.db_trigger;
    let mut settings_save_status = props.settings_save_status;
    let workspace = props.workspace.clone();

    let modules_active_val: serde_json::Value =
        serde_json::from_str(&workspace.modules_active).unwrap_or_default();

    let todos_enabled = modules_active_val
        .get("todos")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let notes_enabled = modules_active_val
        .get("notes")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let reporting_enabled = modules_active_val
        .get("reporting")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let handle_toggle = move |module_key: &'static str, next_state: bool| {
        settings_save_status.set("saving".to_string());
        let mut map = modules_active_val.as_object().cloned().unwrap_or_default();
        map.insert(module_key.to_string(), serde_json::Value::Bool(next_state));

        let new_json = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
        let ws_id = workspace.id.clone();
        let requester_uid = use_context::<crate::state::AppState>()
            .active_user_id
            .read()
            .clone();
        spawn(async move {
            match update_workspace_modules(requester_uid, ws_id, new_json).await {
                Ok(_) => {
                    let current = *db_trigger.read();
                    db_trigger.set(current + 1);
                    settings_save_status.set("saved".to_string());
                }
                Err(e) => {
                    settings_save_status.set(format!("error:{}", e));
                }
            }
        });
    };

    rsx! {
        div { class: "space-y-5 py-2",
            // 1. Todos toggle
            div { class: "flex items-center justify-between gap-4 p-4 rounded-xl border border-border/60 bg-muted/20",
                div { class: "flex-1",
                    h5 { class: "text-sm font-bold text-foreground m-0", "Enable Job Checklist (Todos)" }
                    p { class: "text-xs text-muted-foreground m-0 mt-1 leading-relaxed",
                        "Include an interactive checklist of tasks for every assigned job ticket."
                    }
                }
                components::Switch {
                    checked: todos_enabled,
                    onchange: {
                        let handle_toggle = handle_toggle.clone();
                        move |val| {
                            let mut ht = handle_toggle.clone();
                            ht("todos", val);
                        }
                    }
                }
            }

            // 2. Notes toggle
            div { class: "flex items-center justify-between gap-4 p-4 rounded-xl border border-border/60 bg-muted/20",
                div { class: "flex-1",
                    h5 { class: "text-sm font-bold text-foreground m-0", "Enable Job Notes" }
                    p { class: "text-xs text-muted-foreground m-0 mt-1 leading-relaxed",
                        "Allow adding notes and descriptions to document details for each job."
                    }
                }
                components::Switch {
                    checked: notes_enabled,
                    onchange: {
                        let handle_toggle = handle_toggle.clone();
                        move |val| {
                            let mut ht = handle_toggle.clone();
                            ht("notes", val);
                        }
                    }
                }
            }

            // 3. Reporting toggle
            div { class: "flex items-center justify-between gap-4 p-4 rounded-xl border border-border/60 bg-muted/20",
                div { class: "flex-1",
                    h5 { class: "text-sm font-bold text-foreground m-0", "Enable Job Completion Reporting" }
                    p { class: "text-xs text-muted-foreground m-0 mt-1 leading-relaxed",
                        "Require completion reports to be submitted by the assignee when finishing a job."
                    }
                }
                components::Switch {
                    checked: reporting_enabled,
                    onchange: {
                        let handle_toggle = handle_toggle.clone();
                        move |val| {
                            let mut ht = handle_toggle.clone();
                            ht("reporting", val);
                        }
                    }
                }
            }
        }
    }
}
