use crate::components;
use crate::locales::t;
use crate::state::AppState;
use dioxus::prelude::*;
use yntra_core::TodoItem;

#[derive(Props, Clone)]
pub struct TodosViewProps {
    pub active_user_id: Signal<String>,
    pub auth_region: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for TodosViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TodosView(props: TodosViewProps) -> Element {
    let region = props.auth_region.read().clone();
    let db_trig = *props.db_trigger.read();
    let db_trigger = props.db_trigger;
    let state = use_context::<AppState>();
    
    // Fetch todos from FFI
    let todos_resource = use_resource(move || {
        let _ = db_trig;
        async move {
            yntra_core::get_todos().await.unwrap_or_default()
        }
    });

    let todos = todos_resource.read().clone().unwrap_or_default();
    
    let mut new_todo_text = use_signal(String::new);
    let mut active_filter = use_signal(|| "all".to_string());

    // Filter todos
    let filter = active_filter.read().clone();
    let filtered_todos: Vec<TodoItem> = todos
        .iter()
        .filter(|t| match filter.as_str() {
            "active" => !t.completed,
            "completed" => t.completed,
            _ => true,
        })
        .cloned()
        .collect();

    let tab_items = vec![
        components::tabs::TabItem {
            value: "all".to_string(),
            label: t("todos-filter-all", &region),
            icon: None,
        },
        components::tabs::TabItem {
            value: "active".to_string(),
            label: t("todos-filter-active", &region),
            icon: None,
        },
        components::tabs::TabItem {
            value: "completed".to_string(),
            label: t("todos-filter-completed", &region),
            icon: None,
        },
    ];

    rsx! {
        div {
            class: "mx-auto w-full max-w-3xl animate-in fade-in slide-in-from-top-4 duration-300",
            style: "padding: 2rem; display: flex; flex-direction: column; gap: 1.5rem; box-sizing: border-box;",
            


            // Tabs filter
            components::Tabs {
                tabs: tab_items,
                active_tab: active_filter.read().clone(),
                onchange: move |val| active_filter.set(val),
            }

            // Input Box Card
            components::Card {
                class: "p-4 flex gap-3 items-center border border-border bg-sidebar shadow-md rounded-xl",
                style: "background: var(--bg-sidebar); backdrop-filter: blur(8px);",
                components::Input {
                    placeholder: t("todos-input-placeholder", &region),
                    value: new_todo_text.read().clone(),
                    oninput: move |evt: FormEvent| new_todo_text.set(evt.value()),
                    class: "flex-1 rounded-lg border border-border bg-background px-3.5 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                }
                components::Button {
                    variant: components::ButtonVariant::Primary,
                    class: "shrink-0 gap-1.5 flex items-center justify-center font-bold px-4 py-2.5 rounded-lg text-sm transition-all",
                    onclick: move |_| {
                        let text = new_todo_text.read().trim().to_string();
                        if !text.is_empty() {
                            let workspace_id = state.workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string());
                            let trigger = db_trigger;
                            spawn(async move {
                                let mut trigger = trigger;
                                if yntra_core::add_todo(workspace_id, text).await.is_ok() {
                                    let current = *trigger.read();
                                    trigger.set(current + 1);
                                }
                            });
                            new_todo_text.set(String::new());
                        }
                    },
                    components::LucideIcon { name: "check", size: "14" }
                    "{t(\"todos-add-button\", &region)}"
                }
            }

            // Todos List
            div { class: "flex flex-col gap-3.5",
                if filtered_todos.is_empty() {
                    components::Card {
                        class: "text-center p-12 text-muted-foreground flex flex-col items-center justify-center gap-3 border border-border/40 rounded-xl",
                        components::LucideIcon { name: "check-square", size: "40", class: "icon-muted opacity-40 animate-pulse", }
                        p { class: "text-sm font-semibold m-0", "{t(\"todos-empty-state\", &region)}" }
                    }
                } else {
                    {filtered_todos.into_iter().map(|item| {
                        let todo_id = item.id.clone();
                        let is_completed = item.completed;
                        let text = item.text.clone();
                        let trigger = db_trigger;
                        
                        rsx! {
                            div {
                                key: "{todo_id}",
                                class: "group transition-all duration-200 hover:translate-x-1",
                                components::Card {
                                    style: format!(
                                        "padding: 1.15rem; transition: all 0.2s; border-color: {}; background: {};",
                                        if is_completed { "rgba(16, 185, 129, 0.25)" } else { "var(--border-color)" },
                                        if is_completed { "rgba(16, 185, 129, 0.02)" } else { "var(--bg-card)" }
                                    ),
                                    div { class: "flex items-center gap-3.5 justify-between w-full",
                                        div { class: "flex items-center gap-3 flex-1 min-w-0",
                                            components::Checkbox {
                                                checked: is_completed,
                                                onchange: move |_| {
                                                    let id = todo_id.clone();
                                                    let trig = trigger;
                                                    spawn(async move {
                                                        let mut trig = trig;
                                                        if yntra_core::toggle_todo(id).await.is_ok() {
                                                            let current = *trig.read();
                                                            trig.set(current + 1);
                                                        }
                                                    });
                                                }
                                            }
                                            span {
                                                class: "text-sm truncate select-none cursor-pointer",
                                                style: format!(
                                                    "transition: all 0.25s; {}",
                                                    if is_completed { "text-decoration: line-through; color: var(--text-secondary); opacity: 0.65;" } else { "font-medium; text-foreground;" }
                                                ),
                                                onclick: move |_| {
                                                    let id = item.id.clone();
                                                    let trig = trigger;
                                                    spawn(async move {
                                                        let mut trig = trig;
                                                        if yntra_core::toggle_todo(id).await.is_ok() {
                                                            let current = *trig.read();
                                                            trig.set(current + 1);
                                                        }
                                                    });
                                                },
                                                "{text}"
                                            }
                                        }
                                        div { class: "flex items-center gap-2.5 shrink-0",
                                            if is_completed {
                                                span {
                                                    class: "text-[10px] font-bold uppercase px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                    "Completed"
                                                }
                                            } else {
                                                span {
                                                    class: "text-[10px] font-bold uppercase px-2 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20",
                                                    "Active"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}
