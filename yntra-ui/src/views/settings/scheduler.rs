use crate::components;
use crate::locales::t;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct SchedulerSettingsProps {
    pub settings_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub workspace: yntra_core::Workspace,
    pub locale: String,
}

impl PartialEq for SchedulerSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn SchedulerSettings(props: SchedulerSettingsProps) -> Element {
    let mut settings_save_status = props.settings_save_status;
    let mut db_trigger = props.db_trigger;
    let workspace = props.workspace.clone();

    let settings_val: serde_json::Value = serde_json::from_str(&workspace.settings).unwrap_or_default();

    // Local states matching SchedulerSettings.tsx
    let mut default_calendar_view = use_signal(|| {
        settings_val.get("default_calendar_view").and_then(|v| v.as_str()).unwrap_or("week").to_string()
    });
    
    let local_start_hours = settings_val.get("business_hours")
        .and_then(|bh| bh.get("start"))
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| {
            settings_val.get("start_hour").and_then(|v| v.as_i64()).unwrap_or(7)
        });
    let mut start_hour = use_signal(|| local_start_hours as i32);

    let local_end_hours = settings_val.get("business_hours")
        .and_then(|bh| bh.get("end"))
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| {
            settings_val.get("end_hour").and_then(|v| v.as_i64()).unwrap_or(17)
        });
    let mut end_hour = use_signal(|| local_end_hours as i32);

    let mut calendar_density = use_signal(|| {
        settings_val.get("calendar_density").and_then(|v| v.as_str()).unwrap_or("compact").to_string()
    });



    let state = use_context::<crate::state::AppState>();
    let handle_update_settings = {
        let ws_settings_raw = workspace.settings.clone();
        let ws_id = workspace.id.clone();
        let user_id = state.active_user_id.read().clone();
        move |new_view: Option<String>, new_density: Option<String>, new_start: Option<i32>, new_end: Option<i32>| {
            settings_save_status.set("saving".to_string());
            
            let mut settings_map: serde_json::Value = serde_json::from_str(&ws_settings_raw).unwrap_or_default();
            if let Some(view) = new_view {
                settings_map["default_calendar_view"] = serde_json::json!(view);
            }
            if let Some(density) = new_density {
                settings_map["calendar_density"] = serde_json::json!(density);
            }
            
            let bh_start = new_start.unwrap_or(*start_hour.read());
            let bh_end = new_end.unwrap_or(*end_hour.read());
            
            settings_map["business_hours"] = serde_json::json!({
                "start": bh_start,
                "end": bh_end
            });
            
            let settings_str = serde_json::to_string(&settings_map).unwrap_or_default();
            let ws_id_clone = ws_id.clone();
            let settings_str_clone = settings_str.clone();
            let requester_uid = user_id.clone();
            spawn(async move {
                let _ = yntra_core::update_workspace_settings(requester_uid, ws_id_clone, settings_str_clone).await;
            });

            let current_trig = *db_trigger.read();
            db_trigger.set(current_trig + 1);
            settings_save_status.set("saved".to_string());
        }
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "grid grid-cols-1 gap-6 md:grid-cols-2",
                
                // 1. Calendar Display Card
                components::Card { class: "border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-sm",
                    components::CardHeader {
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-lg bg-primary/10 p-2 text-primary",
                                components::LucideIcon { name: "layout", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg", "{t(\"settings-scheduler-display\", &props.locale)}" }
                                components::CardDescription { "{t(\"settings-scheduler-display-desc\", &props.locale)}" }
                            }
                        }
                    }
                    components::CardContent { class: "space-y-6",
                        div { class: "space-y-2",
                            label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                "{t(\"settings-scheduler-default-view\", &props.locale)}"
                            }
                            select {
                                class: "yntra-input h-10 border-border/50 bg-background/50",
                                value: "{default_calendar_view}",
                                onchange: {
                                    let handle_update_settings = handle_update_settings.clone();
                                    move |e| {
                                        let mut handle_update_settings = handle_update_settings.clone();
                                        let val = e.value();
                                        default_calendar_view.set(val.clone());
                                        handle_update_settings(Some(val), None, None, None);
                                    }
                                },
                                option { value: "day", "{t(\"scheduler-views-day\", &props.locale)}" }
                                option { value: "week", "{t(\"scheduler-views-week\", &props.locale)}" }
                                option { value: "month", "{t(\"scheduler-views-month\", &props.locale)}" }
                                option { value: "agenda", "{t(\"scheduler-views-agenda\", &props.locale)}" }
                            }
                        }

                        div { class: "space-y-3",
                            label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                "{t(\"settings-scheduler-density\", &props.locale)}"
                            }
                            div { class: "grid grid-cols-2 gap-3",
                                button {
                                    class: if *calendar_density.read() == "compact" {
                                        "flex flex-col items-center justify-center rounded-xl border-2 p-3 transition-all border-primary bg-primary/10 text-primary shadow-sm"
                                    } else {
                                        "flex flex-col items-center justify-center rounded-xl border-2 p-3 transition-all border-border/40 bg-background/40 hover:border-primary/40 text-muted-foreground"
                                    },
                                    onclick: {
                                        let handle_update_settings = handle_update_settings.clone();
                                        move |_| {
                                            let mut handle_update_settings = handle_update_settings.clone();
                                            calendar_density.set("compact".to_string());
                                            handle_update_settings(None, Some("compact".to_string()), None, None);
                                        }
                                    },
                                    components::LucideIcon {
                                        name: "monitor",
                                        class: "mb-2 h-4 w-4 rotate-45 scale-75",
                                    }
                                    span { class: "text-xs font-medium",
                                        "{t(\"settings-scheduler-density-compact\", &props.locale)}"
                                    }
                                }
                                button {
                                    class: if *calendar_density.read() == "relaxed" {
                                        "flex flex-col items-center justify-center rounded-xl border-2 p-3 transition-all border-primary bg-primary/10 text-primary shadow-sm"
                                    } else {
                                        "flex flex-col items-center justify-center rounded-xl border-2 p-3 transition-all border-border/40 bg-background/40 hover:border-primary/40 text-muted-foreground"
                                    },
                                    onclick: {
                                        let handle_update_settings = handle_update_settings.clone();
                                        move |_| {
                                            let mut handle_update_settings = handle_update_settings.clone();
                                            calendar_density.set("relaxed".to_string());
                                            handle_update_settings(None, Some("relaxed".to_string()), None, None);
                                        }
                                    },
                                    components::LucideIcon {
                                        name: "monitor",
                                        class: "mb-2 h-4 w-4",
                                    }
                                    span { class: "text-xs font-medium",
                                        "{t(\"settings-scheduler-density-relaxed\", &props.locale)}"
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Working Hours Card
                components::Card { class: "border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-sm",
                    components::CardHeader {
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-lg bg-orange-500/10 p-2 text-orange-500",
                                components::LucideIcon { name: "clock", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg", "{t(\"settings-scheduler-hours\", &props.locale)}" }
                                components::CardDescription { "{t(\"settings-scheduler-hours-desc\", &props.locale)}" }
                            }
                        }
                    }
                    components::CardContent { class: "space-y-6",
                        div { class: "space-y-4",
                            div { class: "space-y-2",
                                div { class: "flex justify-between",
                                    label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                        "{t(\"settings-scheduler-start-hour\", &props.locale)}"
                                    }
                                    span { class: "font-mono text-xs text-foreground", "{start_hour.read()}:00" }
                                }
                                input {
                                    r#type: "range",
                                    min: "0",
                                    max: "12",
                                    step: "1",
                                    class: "w-full cursor-pointer",
                                    value: "{start_hour}",
                                    oninput: {
                                        let handle_update_settings = handle_update_settings.clone();
                                        move |e| {
                                            let mut handle_update_settings = handle_update_settings.clone();
                                            if let Ok(val) = e.value().parse::<i32>() {
                                                start_hour.set(val);
                                                handle_update_settings(None, None, Some(val), None);
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "space-y-2",
                                div { class: "flex justify-between",
                                    label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                        "{t(\"settings-scheduler-end-hour\", &props.locale)}"
                                    }
                                    span { class: "font-mono text-xs text-foreground", "{end_hour.read()}:00" }
                                }
                                input {
                                    r#type: "range",
                                    min: "13",
                                    max: "23",
                                    step: "1",
                                    class: "w-full cursor-pointer",
                                    value: "{end_hour}",
                                    oninput: {
                                        let handle_update_settings = handle_update_settings.clone();
                                        move |e| {
                                            let mut handle_update_settings = handle_update_settings.clone();
                                            if let Ok(val) = e.value().parse::<i32>() {
                                                end_hour.set(val);
                                                handle_update_settings(None, None, None, Some(val));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        p { class: "text-[10px] italic text-muted-foreground m-0",
                            "{t(\"settings-scheduler-hours-info\", &props.locale)}"
                        }
                    }
                }
            }

            // Teaser Card
            components::Card { class: "border-primary/20 bg-primary/[0.03] shadow-none col-span-full",
                components::CardContent { class: "flex items-center gap-4 p-4",
                    div { class: "rounded-lg bg-primary/10 p-2 text-primary flex items-center justify-center",
                        components::LucideIcon { name: "calendar", class: "h-5 w-5" }
                    }
                    div { class: "space-y-1",
                        p { class: "text-sm font-bold text-foreground m-0", "{t(\"settings-scheduler-advanced-title\", &props.locale)}" }
                        p { class: "text-xs text-muted-foreground m-0", "{t(\"settings-scheduler-advanced-desc\", &props.locale)}" }
                    }
                }
            }
        }
    }
}
