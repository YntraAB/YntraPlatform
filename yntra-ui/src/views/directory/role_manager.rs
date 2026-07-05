use dioxus::prelude::*;
use crate::components;
use crate::locales::t;
use crate::state::AppState;
use super::{WorkspaceRole, WorkspaceRolePermissions};

#[component]
pub fn RoleManagerDialog(
    mut show_role_manager_modal: Signal<bool>,
    custom_roles: Vec<WorkspaceRole>,
    settings_val: serde_json::Value,
) -> Element {
    let state = use_context::<AppState>();
    let region = state.auth_region.read().clone();
    let mut db_trigger = state.db_trigger;
    let workspace_id = state.workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string());

    // Parse workspace modules to conditionally show permissions
    let ws_read = state.workspace.read();
    let modules_active_str = ws_read.as_ref().map(|w| w.modules_active.clone()).unwrap_or_default();
    let modules_active: serde_json::Value = serde_json::from_str(&modules_active_str).unwrap_or_default();
    
    let is_school = modules_active.get("school").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_assistance = modules_active.get("assistance").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_active.get("journals").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_active.get("medications").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_moving_company = modules_active.get("moving_company").and_then(|v| v.as_bool()).unwrap_or(false);

    // Form inputs state
    let mut editing_role_id = use_signal(|| Option::<String>::None);
    let mut role_form_name = use_signal(String::new);
    let mut role_form_permissions = use_signal(WorkspaceRolePermissions::default);

    let settings_val_1 = settings_val.clone();
    let settings_val_2 = settings_val.clone();
    let custom_roles_1 = custom_roles.clone();
    let custom_roles_2 = custom_roles.clone();

    rsx! {
        if *show_role_manager_modal.read() {
            components::Dialog {
                open: *show_role_manager_modal.read(),
                title: "Workspace Role & Permission Manager".to_string(),
                max_width: "800px".to_string(),
                onclose: move |_| {
                    show_role_manager_modal.set(false);
                    editing_role_id.set(None);
                },
                div {
                    class: "flex gap-6 w-full text-sm",
                    style: "min-height:420px; width: 100%;",
                    // Left list of roles
                    div {
                        class: "border-r border-border flex flex-col gap-3",
                        style: "width:220px; padding-right:1rem;",
                        // Locked Assistant Role
                        div {
                            class: "p-3 rounded-lg bg-white/[0.02] cursor-pointer",
                            style: "border:1px solid transparent;",
                            style: if editing_role_id.read().is_none() { "border-color:var(--accent-color); background:var(--accent-color-soft);" } else { "" },
                            onclick: move |_| {
                                editing_role_id.set(None);
                                role_form_name.set(String::new());
                                role_form_permissions.set(WorkspaceRolePermissions::default());
                            },
                            div { class: "font-bold",
                            style: "color:var(--text-main);", "Assistant" }
                            div { class: "text-[11px] text-muted-foreground/60",
                            style: "margin-top:0.15rem;", "Locked Default Role" }
                        }
                        // Custom Roles
                        for role in custom_roles.iter() {
                            {
                                let r_id = role.id.clone();
                                let r_name = role.name.clone();
                                let r_clone = role.clone();
                                rsx! {
                                    div {
                                        key: "{r_id}",
                                        class: "p-3 rounded-lg bg-white/[0.02] cursor-pointer",
                                        style: "border:1px solid transparent;",
                                        style: if editing_role_id.read().as_ref() == Some(&r_id) { "border-color:var(--accent-color); background:var(--accent-color-soft);" } else { "" },
                                        onclick: move |_| {
                                            editing_role_id.set(Some(r_id.clone()));
                                            role_form_name.set(r_name.clone());
                                            role_form_permissions.set(r_clone.permissions.clone());
                                        },
                                        div { class: "font-bold",
                                        style: "color:var(--text-main);", "{r_name}" }
                                        div { class: "text-[11px] text-muted-foreground/60",
                                        style: "margin-top:0.15rem;", "Custom Role" }
                                    }
                                }
                            }
                        }
                        button {
                            class: "yntra-btn secondary text-xs",
                            style: "margin-top:auto; padding:0.4rem;",
                            onclick: move |_| {
                                editing_role_id.set(Some("new".to_string()));
                                role_form_name.set(String::new());
                                role_form_permissions.set(WorkspaceRolePermissions::default());
                            },
                            "+ {t(\"directory-role-manager-create-new-button\", &region)}"
                        }
                    }

                    // Right role details/editor
                    div { class: "flex-1 flex flex-col gap-5",
                    style: "padding-left:0.5rem;",
                        if editing_role_id.read().is_none() {
                            // Assistant info view
                            div { class: "flex flex-col gap-4",
                                h4 { class: "m-0 font-extrabold",
                                style: "color:var(--text-main);", "Employee / Team Member" }
                                p { class: "m-0 text-xs text-muted-foreground",
                                style: "line-height:1.4;",
                                    "Locked system employee role. Has view access to schedule, messages, and handover notes, with permissions to submit shift time reports."
                                }
                                div { class: "border-t border-border pt-4 flex flex-col gap-2",
                                    div { class: "text-[11px] font-bold text-muted-foreground/60 uppercase", "Permissions Enabled" }
                                    span { class: "text-muted-foreground/60 text-xs", "• Read Schedule & Notes" }
                                    span { class: "text-muted-foreground/60 text-xs", "• Report Shift Hours" }
                                }
                            }
                        } else {
                            {
                                let is_new = editing_role_id.read().as_ref() == Some(&"new".to_string());
                                rsx! {
                                div { class: "flex flex-col gap-4 flex-1",
                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-xs font-bold text-muted-foreground", "Role Name" }
                                        input {
                                            class: "yntra-input",
                                            value: "{role_form_name}",
                                            placeholder: "e.g. Supervisor / Team Lead",
                                            oninput: move |e| role_form_name.set(e.value()),
                                        }
                                    }
                                    div { class: "flex flex-col gap-4 mt-2 overflow-y-auto max-h-[300px] pr-2 border-t border-b border-border py-4",
                                        
                                        // Section: General Operations
                                        div { class: "flex flex-col gap-2.5",
                                            div { class: "text-[11px] font-bold text-muted-foreground/60 uppercase border-b border-border",
                                            style: "padding-bottom:0.25rem;", "General Operations" }
                                            
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_manage_schedule,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_manage_schedule = val;
                                                    }
                                                }
                                                span { "can_manage_schedule" }
                                            }
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_manage_notes,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_manage_notes = val;
                                                    }
                                                }
                                                span { "can_manage_notes" }
                                            }
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_approve_time_reports,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_approve_time_reports = val;
                                                    }
                                                }
                                                span { "can_approve_time_reports" }
                                            }
                                        }

                                        // Section: System Administration
                                        div { class: "flex flex-col gap-2.5 mt-2",
                                            div { class: "text-[11px] font-bold text-muted-foreground/60 uppercase border-b border-border",
                                            style: "padding-bottom:0.25rem;", "System Administration" }
                                            
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_manage_workspace,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_manage_workspace = val;
                                                    }
                                                }
                                                span { "can_manage_workspace" }
                                            }
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_manage_users,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_manage_users = val;
                                                    }
                                                }
                                                span { "can_manage_users" }
                                            }
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_view_audit_logs,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_view_audit_logs = val;
                                                    }
                                                }
                                                span { "can_view_audit_logs" }
                                            }
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_submit_reports,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_submit_reports = val;
                                                    }
                                                }
                                                span { "can_submit_reports" }
                                            }
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: role_form_permissions.read().can_manage_reports,
                                                    onchange: move |e| {
                                                        let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                        role_form_permissions.write().can_manage_reports = val;
                                                    }
                                                }
                                                span { "can_manage_reports" }
                                            }
                                        }

                                        // Section: School Administration
                                        if is_school {
                                            div { class: "flex flex-col gap-2.5 mt-2",
                                                div { class: "text-[11px] font-bold text-muted-foreground/60 uppercase border-b border-border",
                                                style: "padding-bottom:0.25rem;", "School Administration" }
                                                
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_students,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_students = val;
                                                        }
                                                    }
                                                    span { "can_manage_students" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_grades,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_grades = val;
                                                        }
                                                    }
                                                    span { "can_manage_grades" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_publish_report_cards,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_publish_report_cards = val;
                                                        }
                                                    }
                                                    span { "can_publish_report_cards" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_access_health_records,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_access_health_records = val;
                                                        }
                                                    }
                                                    span { "can_access_health_records" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_billing,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_billing = val;
                                                        }
                                                    }
                                                    span { "can_manage_billing" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_library,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_library = val;
                                                        }
                                                    }
                                                    span { "can_manage_library" }
                                                }
                                            }
                                        }

                                        // Section: Care & Assistance
                                        if is_assistance {
                                            div { class: "flex flex-col gap-2.5 mt-2",
                                                div { class: "text-[11px] font-bold text-muted-foreground/60 uppercase border-b border-border",
                                                style: "padding-bottom:0.25rem;", "Care & Assistance" }
                                                
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_clients,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_clients = val;
                                                        }
                                                    }
                                                    span { "can_manage_clients" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_view_journals,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_view_journals = val;
                                                        }
                                                    }
                                                    span { "can_view_journals" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_write_journals,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_write_journals = val;
                                                        }
                                                    }
                                                    span { "can_write_journals" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_view_medications,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_view_medications = val;
                                                        }
                                                    }
                                                    span { "can_view_medications" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_medications,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_medications = val;
                                                        }
                                                    }
                                                    span { "can_manage_medications" }
                                                }
                                            }
                                        }

                                        // Section: Logistics & Moving
                                        if is_moving_company {
                                            div { class: "flex flex-col gap-2.5 mt-2",
                                                div { class: "text-[11px] font-bold text-muted-foreground/60 uppercase border-b border-border",
                                                style: "padding-bottom:0.25rem;", "Logistics & Moving" }
                                                
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_jobs,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_jobs = val;
                                                        }
                                                    }
                                                    span { "can_manage_jobs" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_manage_quotes,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_manage_quotes = val;
                                                        }
                                                    }
                                                    span { "can_manage_quotes" }
                                                }
                                                label { class: "flex items-center gap-2 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: role_form_permissions.read().can_complete_jobs,
                                                        onchange: move |e| {
                                                            let val = e.value() == "true" || e.value() == "on" || e.value() == "checked";
                                                            role_form_permissions.write().can_complete_jobs = val;
                                                        }
                                                    }
                                                    span { "can_complete_jobs" }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "flex justify-end gap-2 border-t border-border pt-4",
                                    style: "margin-top:auto;",
                                        if !is_new {
                                            button {
                                                class: "yntra-btn secondary text-xs",
                                                style: "color:var(--danger); border-color:rgba(239,68,68,0.2); margin-right:auto; padding:0.4rem 0.8rem;",
                                                onclick: {
                                                    let workspace_id = workspace_id.clone();
                                                    move |_| {
                                                        let current_id = editing_role_id.read().clone().unwrap_or_default();
                                                        let mut roles = custom_roles_1.clone();
                                                        roles.retain(|r| r.id != current_id);
                                                        
                                                        let mut new_settings = settings_val_1.clone();
                                                        new_settings["roles"] = serde_json::to_value(&roles).unwrap();
                                                        let settings_str = serde_json::to_string(&new_settings).unwrap_or_default();
                                                        
                                                        let ws_id = workspace_id.clone();
                                                        let requester_uid = state.active_user_id.read().clone();
                                                        spawn(async move {
                                                            let _ = yntra_core::update_workspace_settings(requester_uid, ws_id, settings_str).await;
                                                        });
                                                        editing_role_id.set(None);
                                                        let current = *db_trigger.read();
                                                        db_trigger.set(current + 1);
                                                    }
                                                },
                                                "Ta bort"
                                            }
                                        }
                                        button {
                                            class: "yntra-btn secondary text-xs",
                                            style: "padding:0.4rem 0.8rem;",
                                            onclick: move |_| {
                                                editing_role_id.set(None);
                                            },
                                            "Avbryt"
                                        }
                                        button {
                                            class: "yntra-btn text-white text-xs",
                                            style: "background: var(--accent); border-color: var(--accent); padding:0.4rem 1.2rem;",
                                            onclick: {
                                                let workspace_id = workspace_id.clone();
                                                move |_| {
                                                    let name_val = role_form_name.read().trim().to_string();
                                                    if !name_val.is_empty() {
                                                        let mut roles = custom_roles_2.clone();
                                                        let perms = role_form_permissions.read().clone();
                                                        
                                                        if is_new {
                                                            let new_role = WorkspaceRole {
                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                name: name_val,
                                                                permissions: perms,
                                                            };
                                                            roles.push(new_role);
                                                        } else {
                                                            let current_id = editing_role_id.read().clone().unwrap_or_default();
                                                            if let Some(r) = roles.iter_mut().find(|r| r.id == current_id) {
                                                                r.name = name_val;
                                                                r.permissions = perms;
                                                            }
                                                        }
                                                        
                                                        let mut new_settings = settings_val_2.clone();
                                                        new_settings["roles"] = serde_json::to_value(&roles).unwrap();
                                                        let settings_str = serde_json::to_string(&new_settings).unwrap_or_default();
                                                        
                                                        let ws_id = workspace_id.clone();
                                                        let requester_uid = state.active_user_id.read().clone();
                                                        spawn(async move {
                                                            let _ = yntra_core::update_workspace_settings(requester_uid, ws_id, settings_str).await;
                                                        });
                                                        editing_role_id.set(None);
                                                        let current = *db_trigger.read();
                                                        db_trigger.set(current + 1);
                                                    }
                                                }
                                            },
                                            if is_new { "Spara ny" } else { "Spara ändringar" }
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
