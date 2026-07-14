use crate::components::{Button, Dialog, DynamicForm, DynamicList, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{
    DynamicEntity, delete_dynamic_entity, get_blocks, get_dynamic_entities, save_dynamic_entity,
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
    let mut db_trigger = props.db_trigger;
    let block_id = props.block_id.clone();
    let workspace_id = props.workspace_id.clone();

    // 1. Fetch block definition to get name, description, fields_schema, ui_config
    let db_trig_val = *db_trigger.read();
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
                "Loading dynamic block metadata..."
            }
        };
    }
    let block = block_opt.unwrap();

    // Schema and UI config fallbacks
    let fields_schema = block.fields_schema.clone().unwrap_or_default();
    let ui_config = block.ui_config.clone().unwrap_or_default();

    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background px-8 py-6",
            // Header
            div { class: "flex items-center justify-between border-b border-border pb-4 mb-6",
                div {
                    h2 { class: "text-lg font-bold text-foreground flex items-center gap-2 m-0",
                        LucideIcon { name: &block.icon, class: "h-5 w-5 text-primary" }
                        "{block.name}"
                    }
                    if let Some(desc) = &block.description {
                        p { class: "text-xs text-muted-foreground mt-1 mb-0", "{desc}" }
                    }
                }
                div {
                    if !fields_schema.is_empty() {
                        Button {
                            class: "flex items-center gap-1.5 text-xs h-9 px-4 rounded-xl",
                            onclick: move |_| {
                                editing_entity.set(None);
                                show_form_modal.set(true);
                            },
                            LucideIcon { name: "plus", class: "h-4 w-4" }
                            "Add Record"
                        }
                    }
                }
            }

            // Body Content
            if fields_schema.is_empty() {
                // Schema editor fallback if no schema is configured
                div { class: "flex flex-col items-center justify-center p-12 border border-dashed border-border rounded-2xl bg-card/10 text-center max-w-lg mx-auto mt-8",
                    LucideIcon { name: "settings", class: "h-10 w-10 text-muted-foreground/40 mb-3" }
                    h3 { class: "text-sm font-bold text-foreground m-0", "No Schema Configured" }
                    p { class: "text-xs text-muted-foreground mt-2 max-w-sm",
                        "Dynamic blocks require a fields schema JSON to render forms and listings. Configure a schema in Settings or initialize a sample now."
                    }
                    Button {
                        class: "mt-4 text-xs h-8 px-4 rounded-lg",
                        onclick: {
                            let block_id = block.id.clone();
                            let requester_uid = props.active_user_id.clone();
                            move |_| {
                                let sample_schema = r#"[{"name":"name","label":"Name","type":"text","required":true,"placeholder":"Enter name"},{"name":"notes","label":"Notes","type":"text","required":false,"placeholder":"Enter notes"}]"#;
                                let sample_ui = r#"[{"name":"name","label":"Name"},{"name":"notes","label":"Notes"}]"#;
                                let req_uid = requester_uid.clone();
                                let b_id = block_id.clone();
                                spawn(async move {
                                    let _ = yntra_core::update_block_schema(req_uid, b_id, sample_schema.to_string(), sample_ui.to_string()).await;
                                });
                                let current = *db_trigger.read();
                                db_trigger.set(current + 1);
                            }
                        },
                        "Initialize Sample Schema"
                    }
                }
            } else {
                DynamicList {
                    entities: entities.clone(),
                    ui_config: ui_config.clone(),
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

            // Dialog / Modal Form
            if *show_form_modal.read() {
                {
                    let is_edit = editing_entity.read().is_some();
                    let entity_ws = workspace_id.clone();
                    let entity_block = block.id.clone();
                    let requester_uid = props.active_user_id.clone();

                    let initial_values_json = match editing_entity.read().as_ref() {
                        Some(e) => e.data.clone(),
                        None => "{}".to_string(),
                    };

                    rsx! {
                        Dialog {
                            open: *show_form_modal.read(),
                            title: if is_edit { "Edit Record" } else { "Add Record" },
                            max_width: "500px".to_string(),
                            onclose: move |_| show_form_modal.set(false),

                            DynamicForm {
                                fields_schema: fields_schema.clone(),
                                initial_values: initial_values_json,
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
        }
    }
}
