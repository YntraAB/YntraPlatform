use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::TeamEvent;
use super::utils::*;

#[derive(Props, Clone)]
pub struct UnscheduledBucketProps {
    pub unscheduled_events: Vec<TeamEvent>,
    pub edit_mode: Signal<bool>,
    pub dragged_event_id: Signal<Option<String>>,
    pub db_trigger: Signal<u32>,
    pub show_event_detail_modal: Signal<Option<TeamEvent>>,
    pub is_admin: bool,
    pub workspace_id: String,
    pub locale: String,
}

impl PartialEq for UnscheduledBucketProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn UnscheduledBucket(props: UnscheduledBucketProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut dragged_event_id = props.dragged_event_id;
    let mut show_event_detail_modal = props.show_event_detail_modal;
    let db_trigger = props.db_trigger;

    let is_editing = *props.edit_mode.read();

    rsx! {
        div {
            class: if is_editing {
                "w-72 border-r border-border bg-sidebar flex flex-col h-full transition-all duration-300 shrink-0"
            } else {
                "w-0 opacity-0 overflow-hidden border-none transition-all duration-300 shrink-0"
            },
            style: "box-sizing: border-box;",
            ondragover: move |e| {
                e.prevent_default();
            },
            ondrop: {
                let mut db_trig = db_trigger;
                let active_uid_sig = state.active_user_id;
                move |e| {
                    e.prevent_default();
                    if let Some(event_id) = dragged_event_id.read().clone() {
                        spawn(async move {
                            let active_uid = active_uid_sig.read().clone();
                            if yntra_core::update_event_time(active_uid, event_id, "unscheduled".to_string(), "unscheduled".to_string()).await.is_ok() {
                                let val = *db_trig.read();
                                db_trig.set(val + 1);
                            }
                        });
                    }
                }
            },
            
            // Header
            div { class: "p-4 border-b border-border flex items-center justify-between bg-background/50",
                h3 { class: "font-semibold text-xs uppercase tracking-widest text-muted-foreground flex items-center gap-2 m-0 select-none",
                    components::LucideIcon { name: "plus", class: "h-4 w-4 text-primary" }
                    "{t(\"scheduler-unscheduled\", &props.locale)}"
                }
                span { class: "bg-primary/10 text-primary text-[10px] px-2 py-0.5 rounded-full font-bold border border-primary/20",
                    "{props.unscheduled_events.len()}"
                }
            }
            
            // Event List
            div { class: "flex-1 overflow-y-auto p-3 flex flex-col gap-2 scrollbar-dark",
                if props.unscheduled_events.is_empty() {
                    div { class: "text-center py-12 px-4 border border-dashed border-border/50 rounded-xl bg-muted/10",
                        components::LucideIcon { name: "calendar", class: "h-8 w-8 mx-auto text-muted-foreground/30 mb-2" }
                        div { class: "text-muted-foreground text-xs italic",
                            "{t(\"scheduler-no-unscheduled-events\", &props.locale)}"
                        }
                    }
                } else {
                    for item in props.unscheduled_events.iter().map(|ev| {
                        let ev_clone = ev.clone();
                        let ev_id = ev.id.clone();
                        let ev_title = ev.title.clone();
                        let config = get_event_category_config(&ev.title, &ev.metadata);
                        let is_dragged = Some(ev_id.clone()) == *dragged_event_id.read();
                        let drag_style = if is_dragged { "opacity: 0.5; transform: scale(0.95);" } else { "" };
                        (ev_clone, ev_id, ev_title, config, drag_style)
                    }) {
                        div {
                            key: "{item.1}",
                            draggable: is_editing,
                            ondragstart: {
                                let ev_id = item.1.clone();
                                move |_| {
                                    dragged_event_id.set(Some(ev_id.clone()));
                                }
                            },
                            ondragend: move |_| {
                                dragged_event_id.set(None);
                            },
                            onclick: {
                                let ev_c = item.0.clone();
                                move |e| {
                                    e.stop_propagation();
                                    show_event_detail_modal.set(Some(ev_c.clone()));
                                }
                            },
                            class: "p-3 rounded-lg border border-border bg-muted cursor-grab active:cursor-grabbing transition-all duration-200 shadow-sm hover:border-primary/50 hover:shadow-md hover:bg-background select-none",
                            style: "border-left: 4px solid {item.3.color}; {item.4}",
                            
                            div { class: "text-sm font-semibold text-foreground truncate flex justify-between items-center gap-1.5",
                                span { "{item.2}" }
                                if is_editing {
                                    button {
                                        r#type: "button",
                                        class: "text-muted-foreground hover:text-destructive bg-transparent border-0 cursor-pointer p-0.5 rounded transition-colors shrink-0",
                                        onclick: {
                                            let ev_id = item.1.clone();
                                            let mut db_trig = db_trigger;
                                            let active_uid_sig = state.active_user_id;
                                            move |evt| {
                                                evt.stop_propagation();
                                                let target_ev_id = ev_id.clone();
                                                spawn(async move {
                                                    let active_uid = active_uid_sig.read().clone();
                                                    let _ = yntra_core::delete_event(active_uid, target_ev_id).await;
                                                    let val = *db_trig.read();
                                                    db_trig.set(val + 1);
                                                });
                                            }
                                        },
                                        components::LucideIcon { name: "trash-2", class: "h-3.5 w-3.5" }
                                    }
                                }
                            }
                            div { 
                                class: "text-[10px] font-bold uppercase mt-1 tracking-wider",
                                style: "color: {item.3.color};",
                                "{t(item.3.label_key, &props.locale)}"
                            }
                        }
                    }
                }
            }
            
            // Bottom Add Button
            if props.is_admin {
                div { class: "p-4 border-t border-border bg-background/30",
                    button {
                        class: "yntra-btn secondary w-full text-xs font-bold py-2 flex items-center justify-center gap-1.5 cursor-pointer hover:border-primary transition-all",
                        style: "border: 1px dashed rgba(255,255,255,0.1);",
                        onclick: {
                            let mut db_trig = db_trigger;
                            let workspace_id = props.workspace_id.clone();
                            let active_uid_sig = state.active_user_id;
                            move |_| {
                                let workspace_id = workspace_id.clone();
                                spawn(async move {
                                    let active_uid = active_uid_sig.read().clone();
                                    let meta_obj = EventMetadata {
                                        category: Some("assistance_time".to_string()),
                                        ..Default::default()
                                    };
                                    let meta_str = serde_json::to_string(&meta_obj).unwrap_or_else(|_| "{}".to_string());
                                    if yntra_core::add_event_with_metadata(
                                        active_uid,
                                        workspace_id,
                                        "Unscheduled Shift".to_string(),
                                        "unscheduled".to_string(),
                                        "unscheduled".to_string(),
                                        None,
                                        None,
                                        None,
                                        meta_str,
                                    ).await.is_ok() {
                                        let val = *db_trig.read();
                                        db_trig.set(val + 1);
                                    }
                                });
                            }
                        },
                        components::LucideIcon { name: "plus", class: "h-3.5 w-3.5" }
                        "{t(\"scheduler-new-shift-button\", &props.locale)}"
                    }
                }
            }
        }
    }
}
