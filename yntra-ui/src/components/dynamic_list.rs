use crate::components::{Button, Dialog, LucideIcon};
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::DynamicEntity;

#[derive(Props, Clone, PartialEq)]
pub struct DynamicListProps {
    pub entities: Vec<DynamicEntity>,
    pub ui_config: String, // JSON Array defining columns
    pub onedit: EventHandler<DynamicEntity>,
    pub ondelete: EventHandler<String>,
    #[props(default = "en".to_string())]
    pub locale: String,
}

#[component]
pub fn DynamicList(props: DynamicListProps) -> Element {
    let loc = props.locale.as_str();

    let ui_config_str = props.ui_config.clone();
    let columns_memo = use_memo(move || {
        let ui_config_val: serde_json::Value = serde_json::from_str(&ui_config_str)
            .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
        ui_config_val.as_array().cloned().unwrap_or_default()
    });
    let columns = columns_memo.read().clone();

    let entities_prop = props.entities.clone();
    let parsed_entities_memo = use_memo(move || {
        entities_prop
            .iter()
            .map(|entity| {
                let entity_data: serde_json::Value =
                    serde_json::from_str(&entity.data).unwrap_or_default();
                (entity.clone(), entity_data)
            })
            .collect::<Vec<(DynamicEntity, serde_json::Value)>>()
    });
    let parsed_entities = parsed_entities_memo.read().clone();

    let mut context_menu_open = use_signal(|| false);
    let mut context_menu_pos = use_signal(|| (0, 0));
    let mut context_menu_entity = use_signal(|| Option::<DynamicEntity>::None);

    let mut pending_delete_id = use_signal(|| Option::<String>::None);

    // Pagination Signals
    let mut current_page = use_signal(|| 1usize);
    let mut page_size = use_signal(|| 25usize);

    let total_items = parsed_entities.len();
    let current_ps = *page_size.read();
    let total_pages = if total_items == 0 {
        1
    } else {
        (total_items + current_ps - 1) / current_ps
    };
    let cur_page = (*current_page.read()).min(total_pages).max(1);

    let start_idx = (cur_page - 1) * current_ps;
    let end_idx = (start_idx + current_ps).min(total_items);

    let page_entities = if start_idx < total_items {
        &parsed_entities[start_idx..end_idx]
    } else {
        &[]
    };

    let display_start = if total_items == 0 { 0 } else { start_idx + 1 };

    rsx! {
        div { class: "w-full overflow-x-auto rounded-lg border border-border bg-card/20 backdrop-blur-md flex flex-col",
            table { class: "w-full border-collapse text-left text-sm text-foreground",
                thead { class: "bg-muted/40 uppercase text-xs font-bold text-muted-foreground border-b border-border",
                    tr {
                        if columns.is_empty() {
                            th { class: "p-4 font-semibold", "{t(\"block-entity-attributes\", loc)}" }
                        } else {
                            for col in columns.iter() {
                                {
                                    let label = col.get("label").and_then(|v| v.as_str()).unwrap_or("Column");
                                    rsx! {
                                        th { class: "p-4 font-semibold", "{label}" }
                                    }
                                }
                            }
                        }
                        th { class: "p-4 text-right", "Actions" }
                    }
                }
                tbody { class: "divide-y divide-border/60",
                    if parsed_entities.is_empty() {
                        tr {
                            td {
                                class: "p-8 text-center text-muted-foreground",
                                colspan: if columns.is_empty() { 2 } else { columns.len() + 1 },
                                "{t(\"common-no-items\", loc)}"
                            }
                        }
                    } else {
                        for (entity , entity_data) in page_entities.iter() {
                            {
                                let e_id = entity.id.clone();
                                let entity_clone = entity.clone();
                                let entity_context = entity.clone();
                                let entity_delete_id = entity.id.clone();
                                rsx! {
                                    tr {
                                        key: "{e_id}",
                                        class: "hover:bg-white/[0.01] transition-colors",
                                        oncontextmenu: move |evt| {
                                            evt.prevent_default();
                                            let coords = evt.client_coordinates();
                                            context_menu_pos.set((coords.x as i32, coords.y as i32));
                                            context_menu_entity.set(Some(entity_context.clone()));
                                            context_menu_open.set(true);
                                        },
                                        if columns.is_empty() {
                                            td { class: "p-4 font-medium text-foreground",
                                                if let Some(obj) = entity_data.as_object() {
                                                    if obj.is_empty() {
                                                        span { class: "text-xs italic text-muted-foreground", "{t(\"block-empty-record\", loc)}" }
                                                    } else {
                                                        div { class: "flex flex-wrap items-center gap-1.5",
                                                            for (key, val) in obj.iter() {
                                                                {
                                                                    let val_str = match val {
                                                                        serde_json::Value::Null => "null".to_string(),
                                                                        serde_json::Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
                                                                        serde_json::Value::Number(n) => n.to_string(),
                                                                        serde_json::Value::String(s) => s.clone(),
                                                                        v => v.to_string(),
                                                                    };
                                                                    rsx! {
                                                                        span {
                                                                            class: "inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium bg-muted/60 text-muted-foreground border border-border/50",
                                                                            span { class: "font-semibold text-foreground/80", "{key}:" }
                                                                            span { "{val_str}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    span { class: "text-xs font-mono text-muted-foreground", "{entity.data}" }
                                                }
                                            }
                                        } else {
                                            for col in columns.iter() {
                                                {
                                                    let col_name = col.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                                                    let val = entity_data.get(col_name).unwrap_or(&serde_json::Value::Null);
                                                    let rendered_val = match val {
                                                        serde_json::Value::Null => String::new(),
                                                        serde_json::Value::Bool(b) => if *b { "Yes".to_string() } else { "No".to_string() },
                                                        serde_json::Value::Number(n) => n.to_string(),
                                                        serde_json::Value::String(s) => s.clone(),
                                                        v => v.to_string(),
                                                    };
                                                    rsx! {
                                                        td { class: "p-4 text-muted-foreground font-medium", "{rendered_val}" }
                                                    }
                                                }
                                            }
                                        }
                                        td { class: "p-4 text-right flex justify-end gap-2",
                                            button {
                                                class: "p-1.5 rounded-lg border border-border/40 hover:bg-white/[0.04] text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                                                onclick: move |_| props.onedit.call(entity_clone.clone()),
                                                LucideIcon { name: "edit-2", class: "h-3.5 w-3.5" }
                                            }
                                            button {
                                                class: "p-1.5 rounded-lg border border-red-500/10 hover:bg-red-500/10 text-red-400 hover:text-red-300 cursor-pointer transition-all",
                                                onclick: move |_| pending_delete_id.set(Some(entity_delete_id.clone())),
                                                LucideIcon { name: "trash-2", class: "h-3.5 w-3.5" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Pagination Controls Footer
            div { class: "flex flex-wrap items-center justify-between gap-4 p-4 border-t border-border/60 bg-muted/20 text-xs text-muted-foreground",
                div { class: "flex items-center gap-3",
                    span { "Items per page:" }
                    crate::components::NativeSelect {
                        class: "py-1 px-2 text-xs bg-background border border-border text-foreground rounded-md cursor-pointer",
                        value: "{current_ps}",
                        onchange: move |evt: FormEvent| {
                            if let Ok(sz) = evt.value().parse::<usize>() {
                                page_size.set(sz);
                                current_page.set(1);
                            }
                        },
                        option { value: "25", "25" }
                        option { value: "50", "50" }
                        option { value: "100", "100" }
                    }
                    span {
                        "Showing {display_start} - {end_idx} of {total_items}"
                    }
                }

                div { class: "flex items-center gap-2",
                    button {
                        class: "p-1.5 rounded-lg border border-border/40 hover:bg-white/[0.04] disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer transition-all",
                        disabled: cur_page <= 1,
                        onclick: move |_| current_page.set(cur_page.saturating_sub(1)),
                        LucideIcon { name: "chevron-left", class: "h-3.5 w-3.5" }
                    }
                    span { class: "font-medium text-foreground px-1",
                        "Page {cur_page} of {total_pages}"
                    }
                    button {
                        class: "p-1.5 rounded-lg border border-border/40 hover:bg-white/[0.04] disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer transition-all",
                        disabled: cur_page >= total_pages,
                        onclick: move |_| current_page.set((cur_page + 1).min(total_pages)),
                        LucideIcon { name: "chevron-right", class: "h-3.5 w-3.5" }
                    }
                }
            }

            // Context Menu Overlay
            if let Some(ent) = context_menu_entity.read().clone() {
                {
                    let ent_edit = ent.clone();
                    let ent_delete = ent.clone();
                    let data_payload = ent.data.clone();

                    rsx! {
                        crate::components::ContextMenu {
                            open: *context_menu_open.read(),
                            x: context_menu_pos.read().0,
                            y: context_menu_pos.read().1,
                            onclose: move |_| context_menu_open.set(false),

                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    props.onedit.call(ent_edit.clone());
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "edit", size: "14" }
                                "{t(\"block-edit-record\", loc)}"
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-destructive flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    pending_delete_id.set(Some(ent_delete.id.clone()));
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "trash-2", size: "14" }
                                "{t(\"block-delete-record\", loc)}"
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    let toast = dioxus_primitives::toast::use_toast();
                                    crate::utils::browser::copy_to_clipboard(data_payload.clone(), Some(toast));
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "copy", size: "14" }
                                "{t(\"block-copy-json\", loc)}"
                            }
                        }
                    }
                }
            }

            // Deletion Confirmation Modal
            if let Some(target_id) = pending_delete_id.read().clone() {
                {
                    let target_id_confirm = target_id.clone();
                    rsx! {
                        Dialog {
                            open: true,
                            title: t("block-confirm-deletion-title", loc),
                            max_width: "420px".to_string(),
                            onclose: move |_| pending_delete_id.set(None),

                            div { class: "flex flex-col gap-4 py-2",
                                div { class: "flex items-start gap-3",
                                    div { class: "p-2 rounded-full bg-red-500/10 text-red-500 shrink-0 mt-0.5",
                                        LucideIcon { name: "alert-triangle", class: "h-5 w-5" }
                                    }
                                    div { class: "flex flex-col gap-1 text-sm",
                                        p { class: "font-semibold text-foreground m-0 text-sm",
                                            "{t(\"block-confirm-deletion-title\", loc)}"
                                        }
                                        p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                                            "{t(\"block-confirm-deletion-desc\", loc)}"
                                        }
                                    }
                                }

                                div { class: "flex gap-2 justify-end mt-2 pt-3 border-t border-border/60",
                                    Button {
                                        variant: crate::components::ButtonVariant::Secondary,
                                        onclick: move |_| pending_delete_id.set(None),
                                        "{t(\"common-cancel\", loc)}"
                                    }
                                    button {
                                        class: "px-4 py-2 text-xs font-semibold rounded-lg bg-red-600 hover:bg-red-700 text-white cursor-pointer transition-colors shadow-sm flex items-center gap-1.5",
                                        onclick: move |_| {
                                            props.ondelete.call(target_id_confirm.clone());
                                            pending_delete_id.set(None);
                                        },
                                        LucideIcon { name: "trash-2", class: "h-3.5 w-3.5" }
                                        "{t(\"block-delete-record\", loc)}"
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
