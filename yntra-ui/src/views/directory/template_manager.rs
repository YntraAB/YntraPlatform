use dioxus::prelude::*;
use crate::components;
use crate::state::AppState;
use crate::locales::t;

#[component]
pub fn TemplateManagerDialog(
    mut show_template_manager_modal: Signal<bool>,
    region: String,
) -> Element {
    let state = use_context::<AppState>();
    let mut db_trigger = state.db_trigger;

    // Local module checkbox states
    let mut temp_messaging = use_signal(|| false);
    let mut temp_scheduling = use_signal(|| false);
    let mut temp_notes = use_signal(|| false);
    let mut temp_time = use_signal(|| false);
    let mut temp_journals = use_signal(|| false);
    let mut temp_medications = use_signal(|| false);
    let mut temp_jobs = use_signal(|| false);
    let mut temp_reporting = use_signal(|| false);
    let mut temp_todos = use_signal(|| false);
    let mut temp_academics = use_signal(|| false);
    let mut temp_attendance = use_signal(|| false);
    let mut temp_finance = use_signal(|| false);
    let mut temp_library = use_signal(|| false);
    let mut temp_moving_company = use_signal(|| false);

    // Care Subtype and Reset Roles
    let mut care_subtype = use_signal(|| "aldreomsorg".to_string());
    let mut reset_roles = use_signal(|| false);

    // Sync from workspace.modules_active when dialog opens
    use_effect(use_reactive(&show_template_manager_modal, move |show| {
        if *show.read() {
            let ws_guard = state.workspace.read();
            let ws = ws_guard.clone().unwrap_or_else(|| yntra_core::Workspace {
                id: "workspace-1".to_string(),
                name: "Yntra Operations Ltd".to_string(),
                modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true,\"todos\":true}".to_string(),
                settings: "{}".to_string(),
                brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
                logo_url: None,
                block_settings: "{}".to_string(),
            });
            let modules_val: serde_json::Value = serde_json::from_str(&ws.modules_active).unwrap_or_default();
            temp_messaging.set(modules_val.get("messaging").and_then(|v| v.as_bool()).unwrap_or(true));
            temp_scheduling.set(modules_val.get("scheduling").and_then(|v| v.as_bool()).unwrap_or(true));
            temp_notes.set(modules_val.get("notes").and_then(|v| v.as_bool()).unwrap_or(true));
            temp_time.set(modules_val.get("time").and_then(|v| v.as_bool()).unwrap_or(true));
            let is_ast = modules_val.get("assistance").and_then(|v| v.as_bool()).unwrap_or(true);
            temp_journals.set(modules_val.get("journals").and_then(|v| v.as_bool()).unwrap_or(is_ast));
            temp_medications.set(modules_val.get("medications").and_then(|v| v.as_bool()).unwrap_or(is_ast));
            temp_jobs.set(modules_val.get("jobs").and_then(|v| v.as_bool()).unwrap_or(true));
            temp_reporting.set(modules_val.get("reporting").and_then(|v| v.as_bool()).unwrap_or(true));
            temp_todos.set(modules_val.get("todos").and_then(|v| v.as_bool()).unwrap_or(true));
            temp_academics.set(modules_val.get("academics").and_then(|v| v.as_bool()).unwrap_or(false));
            temp_attendance.set(modules_val.get("attendance").and_then(|v| v.as_bool()).unwrap_or(false));
            temp_finance.set(modules_val.get("finance").and_then(|v| v.as_bool()).unwrap_or(false));
            temp_library.set(modules_val.get("library").and_then(|v| v.as_bool()).unwrap_or(false));
            temp_moving_company.set(modules_val.get("moving_company").and_then(|v| v.as_bool()).unwrap_or(false));

            care_subtype.set(modules_val.get("care_subtype").and_then(|v| v.as_str()).unwrap_or("aldreomsorg").to_string());
            reset_roles.set(false);
        }
    }));

    rsx! {
        if *show_template_manager_modal.read() {
            components::Dialog {
                open: *show_template_manager_modal.read(),
                title: t("templates-modal-title", &region),
                max_width: "600px".to_string(),
                onclose: move |_| show_template_manager_modal.set(false),
                
                div {
                    class: "flex flex-col gap-6 text-sm",
                    style: "width: 100%;",
                    
                    // Template Preset Selection
                    div {
                        h3 { class: "text-sm font-extrabold text-foreground",
                        style: "margin:0 0 0.75rem 0;", "{t(\"templates-select-preset\", &region)}" }
                        
                        div { class: "flex flex-col gap-3",
                            // Preset 1: Care
                            div {
                                class: "border border-border p-3 rounded-lg cursor-pointer bg-white/[0.02]",
                                style: "transition:all 0.2s;",
                                onclick: move |_| {
                                    temp_messaging.set(true);
                                    temp_scheduling.set(true);
                                    temp_notes.set(true);
                                    temp_journals.set(true);
                                    temp_medications.set(true);
                                    temp_jobs.set(false);
                                    temp_reporting.set(false);
                                    temp_todos.set(true);
                                    temp_academics.set(false);
                                    temp_attendance.set(false);
                                    temp_finance.set(false);
                                    temp_library.set(false);
                                    temp_moving_company.set(false);
                                },
                                div { class: "font-bold text-primary", "{t(\"templates-preset-care\", &region)}" }
                                div { class: "text-xs text-muted-foreground mt-1",
                                style: "line-height: 1.3;", "{t(\"templates-preset-care-desc\", &region)}" }
                            }
                            // Preset 2: Jobs
                            div {
                                class: "border border-border p-3 rounded-lg cursor-pointer bg-white/[0.02]",
                                style: "transition:all 0.2s;",
                                onclick: move |_| {
                                    temp_messaging.set(true);
                                    temp_scheduling.set(true);
                                    temp_notes.set(true);
                                    temp_journals.set(false);
                                    temp_medications.set(false);
                                    temp_jobs.set(true);
                                    temp_reporting.set(false);
                                    temp_todos.set(true);
                                    temp_academics.set(false);
                                    temp_attendance.set(false);
                                    temp_finance.set(false);
                                    temp_library.set(false);
                                    temp_moving_company.set(false);
                                },
                                div { class: "font-bold text-primary", "{t(\"templates-preset-jobs\", &region)}" }
                                div { class: "text-xs text-muted-foreground mt-1",
                                style: "line-height: 1.3;", "{t(\"templates-preset-jobs-desc\", &region)}" }
                            }
                            // Preset 3: Moving Company
                            div {
                                class: "border border-border p-3 rounded-lg cursor-pointer bg-white/[0.02]",
                                style: "transition:all 0.2s;",
                                onclick: move |_| {
                                    temp_messaging.set(true);
                                    temp_scheduling.set(true);
                                    temp_notes.set(true);
                                    temp_journals.set(false);
                                    temp_medications.set(false);
                                    temp_jobs.set(true);
                                    temp_reporting.set(false);
                                    temp_todos.set(true);
                                    temp_academics.set(false);
                                    temp_attendance.set(false);
                                    temp_finance.set(false);
                                    temp_library.set(false);
                                    temp_moving_company.set(true);
                                },
                                 div { class: "font-bold text-primary", "{t(\"templates-preset-moving\", &region)}" }
                                 div { class: "text-xs text-muted-foreground mt-1",
                                 style: "line-height: 1.3;", "{t(\"templates-preset-moving-desc\", &region)}" }
                            }
                            // Preset 4: School
                            div {
                                class: "border border-border p-3 rounded-lg cursor-pointer bg-white/[0.02]",
                                style: "transition:all 0.2s;",
                                onclick: move |_| {
                                    temp_messaging.set(true);
                                    temp_scheduling.set(true);
                                    temp_notes.set(true);
                                    temp_journals.set(false);
                                    temp_medications.set(false);
                                    temp_jobs.set(false);
                                    temp_reporting.set(true);
                                    temp_todos.set(true);
                                    temp_academics.set(true);
                                    temp_attendance.set(true);
                                    temp_finance.set(true);
                                    temp_library.set(true);
                                    temp_moving_company.set(false);
                                },
                                div { class: "font-bold text-primary", "{t(\"templates-preset-school\", &region)}" }
                                div { class: "text-xs text-muted-foreground mt-1",
                                style: "line-height: 1.3;", "{t(\"templates-preset-school-desc\", &region)}" }
                            }
                            // Preset 5: Full Suite
                            div {
                                class: "border border-border p-3 rounded-lg cursor-pointer bg-white/[0.02]",
                                style: "transition:all 0.2s;",
                                onclick: move |_| {
                                    temp_messaging.set(true);
                                    temp_scheduling.set(true);
                                    temp_notes.set(true);
                                    temp_journals.set(true);
                                    temp_medications.set(true);
                                    temp_jobs.set(true);
                                    temp_reporting.set(true);
                                    temp_todos.set(true);
                                    temp_academics.set(true);
                                    temp_attendance.set(true);
                                    temp_finance.set(true);
                                    temp_library.set(true);
                                    temp_moving_company.set(false);
                                },
                                div { class: "font-bold text-primary", "{t(\"templates-preset-full\", &region)}" }
                                div { class: "text-xs text-muted-foreground mt-1",
                                style: "line-height: 1.3;", "{t(\"templates-preset-full-desc\", &region)}" }
                            }
                        }
                    }
                    
                    // Individual module toggles
                    div {
                        h3 { class: "text-sm font-extrabold border-t border-border pt-4 text-foreground",
                        style: "margin:0 0 0.75rem 0;", "{t(\"templates-module-toggles\", &region)}" }
                        
                        div { class: "grid gap-3",
                        style: "grid-template-columns:1fr 1fr;",
                            components::Checkbox {
                                checked: *temp_messaging.read(),
                                onchange: move |val| {
                                    temp_messaging.set(val);
                                    if !val { temp_moving_company.set(false); }
                                },
                                label: t("settings-blocks-messaging-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_scheduling.read(),
                                onchange: move |val| {
                                    temp_scheduling.set(val);
                                    if !val { temp_moving_company.set(false); }
                                },
                                label: t("settings-blocks-scheduling-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_notes.read(),
                                onchange: move |val| {
                                    temp_notes.set(val);
                                    if !val { temp_moving_company.set(false); }
                                },
                                label: t("settings-blocks-notes-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_time.read(),
                                onchange: move |val| {
                                    temp_time.set(val);
                                    if !val { temp_moving_company.set(false); }
                                },
                                label: t("settings-blocks-time-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_journals.read(),
                                onchange: move |val| {
                                    temp_journals.set(val);
                                    if val {
                                        temp_moving_company.set(false);
                                        temp_academics.set(false);
                                        temp_attendance.set(false);
                                        temp_finance.set(false);
                                        temp_library.set(false);
                                    }
                                },
                                label: t("settings-blocks-journals-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_medications.read(),
                                onchange: move |val| {
                                    temp_medications.set(val);
                                    if val {
                                        temp_moving_company.set(false);
                                        temp_academics.set(false);
                                        temp_attendance.set(false);
                                        temp_finance.set(false);
                                        temp_library.set(false);
                                    }
                                },
                                label: t("settings-blocks-medications-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_jobs.read(),
                                onchange: move |val| {
                                    temp_jobs.set(val);
                                    if !val { temp_moving_company.set(false); }
                                },
                                label: t("settings-blocks-jobs-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_reporting.read(),
                                onchange: move |val| {
                                    temp_reporting.set(val);
                                    if val { temp_moving_company.set(false); }
                                },
                                label: t("settings-blocks-reporting-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_todos.read(),
                                onchange: move |val| temp_todos.set(val),
                                label: t("settings-blocks-todos-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_academics.read(),
                                onchange: move |val| {
                                    temp_academics.set(val);
                                    if val {
                                        temp_journals.set(false);
                                        temp_medications.set(false);
                                        temp_moving_company.set(false);
                                    }
                                },
                                label: t("settings-blocks-academics-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_attendance.read(),
                                onchange: move |val| {
                                    temp_attendance.set(val);
                                    if val {
                                        temp_journals.set(false);
                                        temp_medications.set(false);
                                        temp_moving_company.set(false);
                                    }
                                },
                                label: t("settings-blocks-attendance-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_finance.read(),
                                onchange: move |val| {
                                    temp_finance.set(val);
                                    if val {
                                        temp_journals.set(false);
                                        temp_medications.set(false);
                                        temp_moving_company.set(false);
                                    }
                                },
                                label: t("settings-blocks-finance-name", &region)
                            }
                            components::Checkbox {
                                checked: *temp_library.read(),
                                onchange: move |val| {
                                    temp_library.set(val);
                                    if val {
                                        temp_journals.set(false);
                                        temp_medications.set(false);
                                        temp_moving_company.set(false);
                                    }
                                },
                                label: t("settings-blocks-library-name", &region)
                            }
                        }
                    }

                    // Care Subtype Selector if Care Assistance is checked
                    if *temp_journals.read() || *temp_medications.read() {
                        div {
                            class: "flex flex-col gap-1.5 border-t border-border pt-4",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "{t(\"templates-care-subtype-label\", &region)}" }
                            select {
                                class: "yntra-input py-1.5 px-3 text-xs bg-background border border-border text-foreground w-full",
                                value: "{care_subtype}",
                                onchange: move |e| care_subtype.set(e.value()),
                                option { value: "aldreomsorg", "Äldreomsorg / SÄBO (Elderly Care)" }
                                option { value: "lss", "LSS / Personlig assistans (Disability Support)" }
                                option { value: "hvb", "HVB / Stödboende (Youth & Residential)" }
                            }
                        }
                    }

                    // Reset Roles Checkbox
                    div {
                        class: "border-t border-border pt-4",
                        components::Checkbox {
                            checked: *reset_roles.read(),
                            onchange: move |val| reset_roles.set(val),
                            label: t("templates-reset-roles-label", &region)
                        }
                    }
                    
                    // Apply Button
                    button {
                        class: "yntra-btn mt-2 w-full",
                        onclick: {
                            let workspace_id = state.workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string());
                            move |_| {
                                let mut modules_map = serde_json::Map::new();
                                modules_map.insert("messaging".to_string(), serde_json::Value::Bool(*temp_messaging.read()));
                                modules_map.insert("scheduling".to_string(), serde_json::Value::Bool(*temp_scheduling.read()));
                                modules_map.insert("notes".to_string(), serde_json::Value::Bool(*temp_notes.read()));
                                modules_map.insert("time".to_string(), serde_json::Value::Bool(*temp_time.read()));
                                modules_map.insert("assistance".to_string(), serde_json::Value::Bool(*temp_journals.read() || *temp_medications.read()));
                                modules_map.insert("journals".to_string(), serde_json::Value::Bool(*temp_journals.read()));
                                modules_map.insert("medications".to_string(), serde_json::Value::Bool(*temp_medications.read()));
                                modules_map.insert("jobs".to_string(), serde_json::Value::Bool(*temp_jobs.read()));
                                modules_map.insert("reporting".to_string(), serde_json::Value::Bool(*temp_reporting.read()));
                                modules_map.insert("todos".to_string(), serde_json::Value::Bool(*temp_todos.read()));
                                modules_map.insert("academics".to_string(), serde_json::Value::Bool(*temp_academics.read()));
                                modules_map.insert("attendance".to_string(), serde_json::Value::Bool(*temp_attendance.read()));
                                modules_map.insert("finance".to_string(), serde_json::Value::Bool(*temp_finance.read()));
                                modules_map.insert("library".to_string(), serde_json::Value::Bool(*temp_library.read()));

                                
                                modules_map.insert("moving_company".to_string(), serde_json::Value::Bool(*temp_moving_company.read()));
                                if *temp_journals.read() || *temp_medications.read() {
                                    modules_map.insert("care_subtype".to_string(), serde_json::Value::String((*care_subtype.read()).clone()));
                                }
                                modules_map.insert("locale".to_string(), serde_json::Value::String(region.clone()));
                                modules_map.insert("reset_roles".to_string(), serde_json::Value::Bool(*reset_roles.read()));
                                
                                let modules_json = serde_json::to_string(&serde_json::Value::Object(modules_map)).unwrap_or_default();
                                let ws_id = workspace_id.clone();
                                let requester_uid = state.active_user_id.read().clone();
                                spawn(async move {
                                    if yntra_core::update_workspace_modules(requester_uid, ws_id, modules_json).await.is_ok() {
                                        show_template_manager_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                });
                            }
                        },
                        "{t(\"templates-save-config\", &region)}"
                    }
                }
            }
        }
    }
}
