use crate::components::{
    Button, Dialog, DynamicForm, DynamicList, FormFieldSchema, LucideIcon,
    TemplateMarketplace, VisualBlockBuilder,
};
use crate::locales::t;
use crate::utils::DioxusDbObserver;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use yntra_core::{
    delete_dynamic_entity, get_blocks, get_dynamic_entities, register_observer,
    save_dynamic_entity, DynamicEntity,
};

#[derive(Props, Clone, PartialEq)]
pub struct DynamicBlockViewProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub block_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn DynamicBlockView(props: DynamicBlockViewProps) -> Element {
    let loc = props.locale.as_str();
    let mut db_trigger = props.db_trigger;
    let block_id = props.block_id.clone();
    let workspace_id = props.workspace_id.clone();

    // Active view mode: "table", "kanban", "calendar", "builder"
    let mut active_view_mode = use_signal(|| "table".to_string());
    let mut show_marketplace = use_signal(|| false);

    // Direct DatabaseObserver subscription
    let mut observer_trigger = use_signal(|| 0u32);
    let observer_channel = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        (tx, Arc::new(Mutex::new(Some(rx))))
    });

    use_effect({
        let rx_opt_arc = observer_channel.1.clone();
        let tx_channel = observer_channel.0.clone();
        move || {
            let mut rx_opt = rx_opt_arc.lock().unwrap();
            if let Some(mut rx) = rx_opt.take() {
                spawn(async move {
                    while let Some(_evt) = rx.recv().await {
                        let cur = *observer_trigger.read();
                        observer_trigger.set(cur + 1);
                    }
                });
            }
            let observer = Box::new(DioxusDbObserver { tx: tx_channel.clone() });
            register_observer(observer);
        }
    });

    // 1. Fetch block definition
    let db_trig_val = *db_trigger.read() + *observer_trigger.read();
    let block_id_clone = block_id.clone();
    let requester_id = props.active_user_id.clone();
    let block_res = use_resource(move || {
        let _ = db_trig_val;
        let b_id = block_id_clone.clone();
        let r_id = requester_id.clone();
        async move {
            let list = get_blocks(r_id).await.unwrap_or_default();
            list.into_iter().find(|b| b.id == b_id)
        }
    });

    // 2. Fetch entities for this block
    let workspace_id_clone = workspace_id.clone();
    let block_id_clone2 = block_id.clone();
    let requester_id = props.active_user_id.clone();
    let entities_res = use_resource(move || {
        let _ = db_trig_val;
        let ws_id = workspace_id_clone.clone();
        let b_id = block_id_clone2.clone();
        let r_id = requester_id.clone();
        async move {
            get_dynamic_entities(r_id, ws_id, b_id)
                .await
                .unwrap_or_default()
        }
    });

    let block_opt = block_res.read().clone().flatten();
    let entities = entities_res.read().clone().unwrap_or_default();

    // Modal state
    let mut show_form_modal = use_signal(|| false);
    let mut editing_entity = use_signal(|| Option::<DynamicEntity>::None);

    if block_opt.is_none() {
        return rsx! {
            div { class: "p-8 text-center text-muted-foreground",
                "{t(\"block-loading-metadata\", loc)}"
            }
        };
    }
    let block = block_opt.unwrap();

    let fields_schema = block.fields_schema.clone().unwrap_or_default();
    let ui_config = block.ui_config.clone().unwrap_or_default();
    let parsed_fields = FormFieldSchema::parse_json(&fields_schema).unwrap_or_default();

    // Determine status column options for Kanban view
    let status_field = parsed_fields
        .iter()
        .find(|f| f.name == "status" || f.name == "priority" || f.name == "care_level" || f.field_type == crate::components::FieldType::Select);
    
    let kanban_statuses: Vec<String> = status_field
        .and_then(|f| f.options.clone())
        .unwrap_or_else(|| vec!["New".to_string(), "In Progress".to_string(), "Completed".to_string()]);

    let curr_view = active_view_mode.read().clone();

    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background px-8 py-6 gap-6",
            // Header & Multi-View Switcher Bar
            div { class: "flex flex-wrap items-center justify-between gap-4 border-b border-border pb-4",
                div { class: "flex items-center gap-3",
                    div { class: "h-10 w-10 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-xs",
                        LucideIcon { name: &block.icon, class: "h-5 w-5" }
                    }
                    div {
                        h2 { class: "text-lg font-bold text-foreground flex items-center gap-2 m-0",
                            "{block.name}"
                        }
                        if let Some(desc) = &block.description {
                            p { class: "text-xs text-muted-foreground mt-0.5 mb-0", "{desc}" }
                        }
                    }
                }

                // View Mode Tabs & Action Buttons
                div { class: "flex flex-wrap items-center gap-3",
                    // Multi-View Segmented Control
                    div { class: "flex items-center p-1 rounded-xl bg-muted/60 border border-border/80 text-xs font-semibold",
                        button {
                            class: format!(
                                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg transition-all cursor-pointer {}",
                                if curr_view == "table" { "bg-background text-foreground shadow-xs" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            onclick: move |_| active_view_mode.set("table".to_string()),
                            LucideIcon { name: "table", class: "h-3.5 w-3.5" }
                            "Table"
                        }
                        button {
                            class: format!(
                                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg transition-all cursor-pointer {}",
                                if curr_view == "kanban" { "bg-background text-foreground shadow-xs" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            onclick: move |_| active_view_mode.set("kanban".to_string()),
                            LucideIcon { name: "kanban", class: "h-3.5 w-3.5" }
                            "Kanban"
                        }
                        button {
                            class: format!(
                                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg transition-all cursor-pointer {}",
                                if curr_view == "calendar" { "bg-background text-foreground shadow-xs" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            onclick: move |_| active_view_mode.set("calendar".to_string()),
                            LucideIcon { name: "calendar", class: "h-3.5 w-3.5" }
                            "Calendar"
                        }
                    }

                    // Admin Visual Builder & Marketplace Triggers
                    div { class: "flex items-center gap-2",
                        button {
                            class: "flex items-center gap-1.5 text-xs h-9 px-3 rounded-xl border border-border bg-secondary text-secondary-foreground hover:bg-secondary/80 transition-all cursor-pointer",
                            title: "Open Visual Schema Builder",
                            onclick: move |_| active_view_mode.set(if curr_view == "builder" { "table".to_string() } else { "builder".to_string() }),
                            LucideIcon { name: "wrench", class: "h-4 w-4 text-primary" }
                            if curr_view == "builder" { "Close Builder" } else { "Visual Builder" }
                        }

                        button {
                            class: "flex items-center gap-1.5 text-xs h-9 px-3 rounded-xl border border-primary/30 bg-primary/10 text-primary hover:bg-primary/20 transition-all cursor-pointer font-semibold",
                            title: "Open 1-Click Industry Template Store",
                            onclick: move |_| show_marketplace.set(true),
                            LucideIcon { name: "store", class: "h-4 w-4" }
                            "Template Store"
                        }

                        if !fields_schema.is_empty() {
                            Button {
                                class: "flex items-center gap-1.5 text-xs h-9 px-4 rounded-xl shadow-md shadow-primary/20",
                                onclick: move |_| {
                                    editing_entity.set(None);
                                    show_form_modal.set(true);
                                },
                                LucideIcon { name: "plus", class: "h-4 w-4" }
                                "{t(\"block-add-record\", loc)}"
                            }
                        }
                    }
                }
            }

            // Main Dynamic View Body
            div { class: "flex-1 overflow-hidden flex flex-col",
                if fields_schema.is_empty() && curr_view != "builder" {
                    // Empty Schema Placeholder State
                    div { class: "flex flex-col items-center justify-center p-12 border border-dashed border-border rounded-3xl bg-card/10 text-center max-w-md mx-auto my-auto space-y-4",
                        div { class: "h-16 w-16 rounded-3xl bg-primary/10 text-primary flex items-center justify-center shadow-inner",
                            LucideIcon { name: "layers", class: "h-8 w-8" }
                        }
                        h3 { class: "text-base font-bold text-foreground m-0", "{t(\"block-no-schema-title\", loc)}" }
                        p { class: "text-xs text-muted-foreground max-w-sm m-0 leading-relaxed",
                            "No schema is configured for this block yet. Use our drag-and-drop Visual Builder or install a 1-click Industry Template!"
                        }
                        div { class: "flex items-center gap-3 pt-2",
                            Button {
                                class: "text-xs h-9 px-4 rounded-xl bg-primary text-primary-foreground",
                                onclick: move |_| active_view_mode.set("builder".to_string()),
                                LucideIcon { name: "wrench", class: "h-4 w-4 mr-1.5" }
                                "Open Visual Builder"
                            }
                            Button {
                                class: "text-xs h-9 px-4 rounded-xl border border-border bg-secondary text-secondary-foreground",
                                onclick: move |_| show_marketplace.set(true),
                                LucideIcon { name: "store", class: "h-4 w-4 mr-1.5" }
                                "Browse Templates"
                            }
                        }
                    }
                } else {
                    match curr_view.as_str() {
                        "builder" => rsx! {
                        VisualBlockBuilder {
                            active_user_id: props.active_user_id.clone(),
                            block_id: block.id.clone(),
                            initial_schema_json: fields_schema.clone(),
                            initial_ui_config_json: ui_config.clone(),
                            locale: props.locale.clone(),
                            onsave: move |_| {
                                active_view_mode.set("table".to_string());
                                let current = *db_trigger.read();
                                db_trigger.set(current + 1);
                            },
                            oncancel: move |_| active_view_mode.set("table".to_string()),
                        }
                    },

                    "kanban" => rsx! {
                        // Dynamic Kanban Board View
                        div { class: "flex-1 overflow-x-auto pb-4",
                            div { class: "flex gap-5 h-full min-w-max",
                                for status in kanban_statuses.iter() {
                                    {
                                        let status_name = status.clone();
                                        // Filter entities matching status
                                        let column_entities: Vec<DynamicEntity> = entities.iter().filter(|e| {
                                            if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&e.data) {
                                                if let Some(val) = map.get("status").or_else(|| map.get("priority")).or_else(|| map.get("care_level")) {
                                                    return val.as_str().unwrap_or("") == status_name;
                                                }
                                            }
                                            false
                                        }).cloned().collect();

                                        rsx! {
                                            div {
                                                key: "{status_name}",
                                                class: "w-80 flex flex-col rounded-2xl border border-border bg-card/40 p-4 space-y-3 shadow-xs shrink-0",
                                                
                                                div { class: "flex items-center justify-between border-b border-border/60 pb-3",
                                                    h4 { class: "text-xs font-bold uppercase tracking-wider text-foreground m-0 flex items-center gap-2",
                                                        span { class: "h-2 w-2 rounded-full bg-primary" }
                                                        "{status_name}"
                                                    }
                                                    span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold bg-muted text-muted-foreground",
                                                        "{column_entities.len()}"
                                                    }
                                                }

                                                div { class: "flex-1 overflow-y-auto space-y-3 pr-1",
                                                    if column_entities.is_empty() {
                                                        div { class: "p-4 text-center text-xs text-muted-foreground italic border border-dashed border-border/60 rounded-xl",
                                                            "No items in {status_name}"
                                                        }
                                                    } else {
                                                        for item in column_entities.iter() {
                                                            {
                                                                let item_id = item.id.clone();
                                                                let item_entity = item.clone();
                                                                let data_val: serde_json::Value = serde_json::from_str(&item.data).unwrap_or_default();
                                                                let title_text = data_val.get("client_name")
                                                                    .or_else(|| data_val.get("patient_name"))
                                                                    .or_else(|| data_val.get("item_name"))
                                                                    .or_else(|| data_val.get("property_unit"))
                                                                    .or_else(|| data_val.get("vehicle_reg"))
                                                                    .or_else(|| data_val.get("name"))
                                                                    .or_else(|| data_val.get("title"))
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("Record Item")
                                                                    .to_string();

                                                                rsx! {
                                                                    div {
                                                                        key: "{item.id}",
                                                                        class: "rounded-xl border border-border bg-background p-4 space-y-3 shadow-xs hover:border-primary/40 transition-all group",
                                                                        
                                                                        div { class: "flex items-start justify-between gap-2",
                                                                            p { class: "text-xs font-bold text-foreground m-0 group-hover:text-primary transition-colors", "{title_text}" }
                                                                            button {
                                                                                class: "p-1 rounded-md text-muted-foreground hover:text-foreground cursor-pointer",
                                                                                onclick: move |_| {
                                                                                    editing_entity.set(Some(item_entity.clone()));
                                                                                    show_form_modal.set(true);
                                                                                },
                                                                                LucideIcon { name: "edit-2", class: "h-3.5 w-3.5" }
                                                                            }
                                                                        }

                                                                        // Render first 2 fields snippet
                                                                        if let Some(obj) = data_val.as_object() {
                                                                            div { class: "space-y-1 text-[11px]",
                                                                                for (k, v) in obj.iter().take(3) {
                                                                                    if k != "status" && k != "title" && k != "client_name" {
                                                                                        {
                                                                                            let v_str = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
                                                                                            rsx! {
                                                                                                div { key: "{k}", class: "flex items-center justify-between text-muted-foreground truncate",
                                                                                                    span { class: "font-semibold capitalize text-[10px]", "{k}:" }
                                                                                                    span { class: "truncate max-w-[120px] font-mono text-[10px]", "{v_str}" }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }

                                                                        // Quick Column Movement Actions
                                                                        div { class: "flex items-center justify-end gap-1 pt-2 border-t border-border/40",
                                                                            for target_st in kanban_statuses.iter() {
                                                                                if target_st != &status_name {
                                                                                    {
                                                                                        let target_st_clone = target_st.clone();
                                                                                        let item_clone = item.clone();
                                                                                        let req_uid = props.active_user_id.clone();
                                                                                        rsx! {
                                                                                            button {
                                                                                                key: "{target_st}",
                                                                                                class: "px-2 py-0.5 rounded-md text-[9px] font-bold uppercase bg-secondary text-secondary-foreground hover:bg-primary/10 hover:text-primary transition-all cursor-pointer",
                                                                                                onclick: move |_| {
                                                                                                    let mut item_mut = item_clone.clone();
                                                                                                    if let Ok(mut map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&item_mut.data) {
                                                                                                        map.insert("status".to_string(), serde_json::Value::String(target_st_clone.clone()));
                                                                                                        map.insert("priority".to_string(), serde_json::Value::String(target_st_clone.clone()));
                                                                                                        map.insert("care_level".to_string(), serde_json::Value::String(target_st_clone.clone()));
                                                                                                        item_mut.data = serde_json::to_string(&map).unwrap_or(item_mut.data);
                                                                                                        let r_uid = req_uid.clone();
                                                                                                        spawn(async move {
                                                                                                            let _ = save_dynamic_entity(r_uid, item_mut).await;
                                                                                                        });
                                                                                                        let cur = *db_trigger.read();
                                                                                                        db_trigger.set(cur + 1);
                                                                                                    }
                                                                                                },
                                                                                                "-> {target_st}"
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
                                    }
                                }
                            }
                        }
                    },

                    "calendar" => rsx! {
                        // Dynamic Calendar Grid View
                        div { class: "flex-1 rounded-2xl border border-border bg-card p-6 shadow-sm overflow-y-auto space-y-4",
                            div { class: "flex items-center justify-between border-b border-border pb-3",
                                h3 { class: "text-sm font-bold text-foreground flex items-center gap-2 m-0",
                                    LucideIcon { name: "calendar", class: "h-5 w-5 text-primary" }
                                    "Monthly Calendar Grid View (August 2026)"
                                }
                                span { class: "text-xs font-semibold text-muted-foreground", "{entities.len()} Total Scheduled Items" }
                            }

                            div { class: "grid grid-cols-7 gap-2 text-center text-xs font-bold text-muted-foreground uppercase pb-2 border-b border-border/40",
                                div { "Mon" }
                                div { "Tue" }
                                div { "Wed" }
                                div { "Thu" }
                                div { "Fri" }
                                div { "Sat" }
                                div { "Sun" }
                            }

                            div { class: "grid grid-cols-7 gap-2.5",
                                for day in 1..=31 {
                                    {
                                        let day_str = format!("2026-08-{:02}", day);
                                        let day_entities: Vec<DynamicEntity> = entities.iter().filter(|e| {
                                            if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&e.data) {
                                                if let Some(d_val) = map.get("service_date").or_else(|| map.get("reported_date")).or_else(|| map.get("start_time")) {
                                                    return d_val.as_str().unwrap_or("").contains(&format!("{:02}", day));
                                                }
                                            }
                                            false
                                        }).cloned().collect();

                                        rsx! {
                                            div {
                                                key: "{day}",
                                                class: format!(
                                                    "min-h-[100px] p-2 rounded-xl border flex flex-col justify-between transition-all {}",
                                                    if !day_entities.is_empty() { "bg-primary/5 border-primary/30" } else { "bg-background border-border/60" }
                                                ),
                                                
                                                div { class: "flex items-center justify-between text-xs font-bold text-foreground",
                                                    span { class: "h-6 w-6 rounded-full flex items-center justify-center bg-muted/60 text-[11px]", "{day}" }
                                                    if !day_entities.is_empty() {
                                                        span { class: "h-2 w-2 rounded-full bg-emerald-500" }
                                                    }
                                                }

                                                div { class: "space-y-1 mt-1 flex-1 overflow-hidden",
                                                    for item in day_entities.iter() {
                                                        {
                                                            let data_val: serde_json::Value = serde_json::from_str(&item.data).unwrap_or_default();
                                                            let item_title = data_val.get("client_name")
                                                                .or_else(|| data_val.get("patient_name"))
                                                                .or_else(|| data_val.get("item_name"))
                                                                .or_else(|| data_val.get("title"))
                                                                .and_then(|v| v.as_str())
                                                                .unwrap_or("Item")
                                                                .to_string();

                                                            rsx! {
                                                                div {
                                                                    key: "{item.id}",
                                                                    class: "px-1.5 py-0.5 rounded bg-primary/20 text-primary text-[10px] font-semibold truncate border border-primary/30",
                                                                    "{item_title}"
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
                    },

                    _ => rsx! {
                        // Standard Table View
                        DynamicList {
                            entities: entities.clone(),
                            ui_config: ui_config.clone(),
                            locale: props.locale.clone(),
                            onedit: move |entity: DynamicEntity| {
                                editing_entity.set(Some(entity));
                                show_form_modal.set(true);
                            },
                            ondelete: {
                                let requester_uid = props.active_user_id.clone();
                                move |id: String| {
                                    let req_uid = requester_uid.clone();
                                    spawn(async move {
                                        let _ = delete_dynamic_entity(req_uid, id).await;
                                    });
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                }
                            }
                        }
                    }
                }
            }
            }

            // Dialog / Modal Form
            if *show_form_modal.read() {
                {
                    let is_edit = editing_entity.read().is_some();
                    let entity_ws = workspace_id.clone();
                    let entity_block = block.id.clone();
                    let requester_uid = props.active_user_id.clone();
                    let dialog_title = if is_edit { t("block-edit-record", loc) } else { t("block-add-record", loc) };

                    let initial_values_json = match editing_entity.read().as_ref() {
                        Some(e) => e.data.clone(),
                        None => "{}".to_string(),
                    };
                    let schema_vec = FormFieldSchema::parse_json(&fields_schema).unwrap_or_default();

                    rsx! {
                        Dialog {
                            open: *show_form_modal.read(),
                            title: dialog_title,
                            max_width: "500px".to_string(),
                            onclose: move |_| show_form_modal.set(false),

                            DynamicForm {
                                fields_schema: schema_vec,
                                initial_values: initial_values_json,
                                locale: props.locale.clone(),
                                onsubmit: move |values_json| {
                                    let ent = DynamicEntity {
                                        id: match editing_entity.read().as_ref() {
                                            Some(e) => e.id.clone(),
                                            None => uuid::Uuid::new_v4().to_string(),
                                        },
                                        workspace_id: entity_ws.clone(),
                                        block_id: entity_block.clone(),
                                        entity_type: "custom".to_string(),
                                        data: values_json,
                                        created_at: match editing_entity.read().as_ref() {
                                            Some(e) => e.created_at,
                                            None => 0,
                                        },
                                        updated_at: 0,
                                        sync_status: "pending".to_string(),
                                    };
                                    let req_uid = requester_uid.clone();
                                    spawn(async move {
                                        let _ = save_dynamic_entity(req_uid, ent).await;
                                    });
                                    show_form_modal.set(false);
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                },
                                oncancel: move |_| show_form_modal.set(false)
                            }
                        }
                    }
                }
            }

            // 1-Click Industry Template Marketplace Modal
            if *show_marketplace.read() {
                TemplateMarketplace {
                    active_user_id: props.active_user_id.clone(),
                    workspace_id: workspace_id.clone(),
                    locale: props.locale.clone(),
                    oninstall: move |_installed_block_id| {
                        show_marketplace.set(false);
                        let current = *db_trigger.read();
                        db_trigger.set(current + 1);
                    },
                    onclose: move |_| show_marketplace.set(false),
                }
            }
        }
    }
}
