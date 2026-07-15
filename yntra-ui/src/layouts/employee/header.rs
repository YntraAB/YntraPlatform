use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::Workspace;
use yntra_core::WorkspaceUser;

#[derive(Props, Clone)]
pub struct LayoutHeaderProps {
    pub active_user: WorkspaceUser,
    pub auth_region: Signal<String>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub header_profile_open: Signal<bool>,
    pub logged_in: Signal<bool>,
    pub db_trigger: Signal<u32>,
    pub workspace: Workspace,
}

impl PartialEq for LayoutHeaderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn LayoutHeader(props: LayoutHeaderProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_user_role = state.active_user_role.read().clone();
    let locale = state.auth_region.read().clone();
    let teams = state.teams.read().clone().unwrap_or_default();

    let section = state.active_section.read().clone();
    let mut notes = Vec::new();
    if section == "notes" {
        notes = state.notes.read().clone().unwrap_or_default();
    }
    let mut messages = Vec::new();
    if section == "messaging" {
        messages = state.messages.read().clone().unwrap_or_default();
    }
    let mut workspaces = Vec::new();
    if section == "directory" {
        workspaces = state.workspaces.read().clone().unwrap_or_default();
    }

    let breadcrumbs = super::breadcrumbs::get_breadcrumbs(
        state.active_section,
        state.selected_note_team_id,
        state.selected_note_id,
        state.is_note_composing,
        state.active_message_id,
        state.messaging_view_tab,
        state.selected_directory_team,
        state.directory_level,
        state.selected_directory_workspace,
        state.selected_calendar_date,
        &active_user_role,
        &locale,
        &teams,
        &notes,
        &messages,
        &workspaces,
    );

    let runner = crate::utils::use_action_runner();
    let active_user = props.active_user.clone();
    let current_role = active_user.role.clone();
    let region = {
        let user_prefs: serde_json::Value =
            serde_json::from_str(&active_user.preferences).unwrap_or_default();
        user_prefs.get("language").and_then(|l| l.as_str()).unwrap_or("US").to_string()
    };

    let active_user_id = props.active_user_id;
    let mut active_section = props.active_section;
    let mut header_profile_open = props.header_profile_open;
    let mut logged_in = props.logged_in;
    let mut db_trigger = props.db_trigger;
    let auth_region = props.auth_region;

    let is_client = current_role == "client";
    let mut header_role_open = use_signal(|| false);
    let mut header_template_open = use_signal(|| false);

    // Global reporting modal signals
    let mut show_reporting_modal = use_signal(|| false);
    let mut report_type = use_signal(|| "incident".to_string());
    let mut report_date = use_signal(|| chrono::Local::now().format("%Y-%m-%d").to_string());
    let mut report_subject = use_signal(String::new);
    let mut report_description = use_signal(String::new);
    let mut report_is_anonymous = use_signal(|| false);
    let report_tab = use_signal(|| "send".to_string());

    let active_template_label = {
        let modules_val: serde_json::Value =
            serde_json::from_str(&props.workspace.modules_active).unwrap_or_default();
        let is_school = modules_val
            .get("school")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || modules_val
                .get("academics")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("attendance")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("finance")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("library")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("timetable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let is_assistance = modules_val
            .get("assistance")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || modules_val
                .get("journals")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("medications")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let is_moving_company = modules_val
            .get("moving_company")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_school {
            "Dev: School".to_string()
        } else if is_moving_company {
            "Dev: Operations".to_string()
        } else if is_assistance {
            "Dev: Care".to_string()
        } else {
            "Dev: General".to_string()
        }
    };

    rsx! {
        header { class: "flex h-14 items-center justify-between border-b border-border bg-sidebar px-6",
            div { class: "flex items-center gap-1.5 text-sm text-muted-foreground duration-200 animate-in fade-in slide-in-from-left-2",
                for (idx, item) in breadcrumbs.iter().enumerate() {
                    {
                        let is_last = idx == breadcrumbs.len() - 1;
                        let item_clone = item.clone();
                        rsx! {
                            if idx > 0 {
                                span { class: "text-muted-foreground/30 font-medium", "/" }
                            }
                            if is_last {
                                span { class: "text-foreground font-semibold capitalize", "{item.label}" }
                            } else {
                                span {
                                    class: "cursor-pointer hover:text-foreground transition-colors capitalize",
                                    onclick: move |_| item_clone.onclick.call(()),
                                    "{item.label}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "flex items-center gap-4",
                components::Dropdown {
                    label: match current_role.as_str() {
                        "platform_admin" => "Platform Admin".to_string(),
                        "admin" => "Admin View".to_string(),
                        "assistant" => "Assistant View".to_string(),
                        "user" => "User View".to_string(),
                        "client" => "Client Portal".to_string(),
                        "technician" => "Technician View".to_string(),
                        _ => current_role.clone(),
                    },
                    open: *header_role_open.read(),
                    ontoggle: move |_| {
                        let current = *header_role_open.read();
                        header_role_open.set(!current);
                    },
                    components::DropdownItem {
                        label: "Platform Admin".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            let runner = runner.clone();
                            move |_| {
                                let user_id = user_id.clone();
                                let runner = runner.clone();
                                runner.run(async move {
                                    yntra_core::update_user_role(user_id.clone(), user_id.clone(), "platform_admin".to_string()).await?;
                                    if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                        active_section.set("dashboard".to_string());
                                    }
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                    Ok(())
                                });
                                header_role_open.set(false);
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Admin View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            let runner = runner.clone();
                            move |_| {
                                let user_id = user_id.clone();
                                let runner = runner.clone();
                                runner.run(async move {
                                    yntra_core::update_user_role(user_id.clone(), user_id.clone(), "admin".to_string()).await?;
                                    if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                        active_section.set("dashboard".to_string());
                                    }
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                    Ok(())
                                });
                                header_role_open.set(false);
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Assistant View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            let runner = runner.clone();
                            move |_| {
                                let user_id = user_id.clone();
                                let runner = runner.clone();
                                runner.run(async move {
                                    yntra_core::update_user_role(user_id.clone(), user_id.clone(), "assistant".to_string()).await?;
                                    if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                        active_section.set("dashboard".to_string());
                                    }
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                    Ok(())
                                });
                                header_role_open.set(false);
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "User View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            let runner = runner.clone();
                            move |_| {
                                let user_id = user_id.clone();
                                let runner = runner.clone();
                                runner.run(async move {
                                    yntra_core::update_user_role(user_id.clone(), user_id.clone(), "user".to_string()).await?;
                                    if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                        active_section.set("dashboard".to_string());
                                    }
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                    Ok(())
                                });
                                header_role_open.set(false);
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Client Portal".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            let runner = runner.clone();
                            move |_| {
                                let user_id = user_id.clone();
                                let runner = runner.clone();
                                runner.run(async move {
                                    yntra_core::update_user_role(user_id.clone(), user_id.clone(), "client".to_string()).await?;
                                    active_section.set("client_portal".to_string());
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                    Ok(())
                                });
                                header_role_open.set(false);
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Technician View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            let runner = runner.clone();
                            move |_| {
                                let user_id = user_id.clone();
                                let runner = runner.clone();
                                runner.run(async move {
                                    yntra_core::update_user_role(user_id.clone(), user_id.clone(), "technician".to_string()).await?;
                                    active_section.set("jobs".to_string());
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                    Ok(())
                                });
                                header_role_open.set(false);
                            }
                        }
                    }
                }
                if current_role == "platform_admin" || current_role == "admin" {
                    components::Dropdown {
                        label: active_template_label,
                        open: *header_template_open.read(),
                        ontoggle: move |_| {
                            let current = *header_template_open.read();
                            header_template_open.set(!current);
                        },
                        components::DropdownItem {
                            label: "School Template".to_string(),
                            onclick: {
                                let user_id = active_user_id.read().clone();
                                let ws_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                let runner = runner.clone();
                                move |_| {
                                    let user_id = user_id.clone();
                                    let ws_id = ws_id.clone();
                                    let runner = runner.clone();
                                    runner.run(async move {
                                        let school_modules = serde_json::json!({
                                            "academics": true,
                                            "attendance": true,
                                            "finance": true,
                                            "library": true,
                                            "timetable": true,
                                            "todos": true,
                                            "messaging": true,
                                            "scheduling": true,
                                            "notes": true,
                                            "reporting": true,
                                            "assistance": false,
                                            "medications": false,
                                            "journals": false,
                                            "time": false,
                                            "jobs": false,
                                        });
                                        yntra_core::update_workspace_modules(user_id, ws_id, school_modules.to_string()).await?;
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                        Ok(())
                                    });
                                    header_template_open.set(false);
                                }
                            }
                        }
                        components::DropdownItem {
                            label: "Care & HVB/LSS".to_string(),
                            onclick: {
                                let user_id = active_user_id.read().clone();
                                let ws_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                let runner = runner.clone();
                                move |_| {
                                    let user_id = user_id.clone();
                                    let ws_id = ws_id.clone();
                                    let runner = runner.clone();
                                    runner.run(async move {
                                        let care_modules = serde_json::json!({
                                            "assistance": true,
                                            "medications": true,
                                            "journals": true,
                                            "time": true,
                                            "jobs": true,
                                            "todos": true,
                                            "messaging": true,
                                            "notes": true,
                                            "reporting": true,
                                            "academics": false,
                                            "attendance": false,
                                            "finance": false,
                                            "library": false,
                                            "timetable": false,
                                        });
                                        yntra_core::update_workspace_modules(user_id, ws_id, care_modules.to_string()).await?;
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                        Ok(())
                                    });
                                    header_template_open.set(false);
                                }
                            }
                        }
                        components::DropdownItem {
                            label: "General Operations".to_string(),
                            onclick: {
                                let user_id = active_user_id.read().clone();
                                let ws_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                let runner = runner.clone();
                                move |_| {
                                    let user_id = user_id.clone();
                                    let ws_id = ws_id.clone();
                                    let runner = runner.clone();
                                    runner.run(async move {
                                        let general_modules = serde_json::json!({
                                            "jobs": true,
                                            "time": true,
                                            "todos": true,
                                            "messaging": true,
                                            "notes": true,
                                            "reporting": true,
                                            "academics": false,
                                            "attendance": false,
                                            "finance": false,
                                            "library": false,
                                            "timetable": false,
                                            "assistance": false,
                                            "medications": false,
                                            "journals": false,
                                        });
                                        yntra_core::update_workspace_modules(user_id, ws_id, general_modules.to_string()).await?;
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                        Ok(())
                                    });
                                    header_template_open.set(false);
                                }
                            }
                        }
                    }
                }

                // Global Quick Action for Incident Reporting
                if !is_client {
                    button {
                        class: "yntra-btn secondary flex items-center gap-2 border border-border bg-transparent text-xs text-foreground hover:bg-white/[0.04] px-3 py-1.5 rounded-lg font-semibold cursor-pointer mr-2",
                        onclick: move |_| {
                            report_subject.set(String::new());
                            report_description.set(String::new());
                            report_is_anonymous.set(false);
                            report_type.set("incident".to_string());
                            report_date.set(chrono::Local::now().format("%Y-%m-%d").to_string());
                            show_reporting_modal.set(true);
                        },
                        components::LucideIcon { name: "shield", size: "14", class: "text-red-400" }
                        span { "Report Incident" }
                    }
                }

                // Header User Profile trigger
                div { class: "relative flex items-center gap-3",
                    div {
                        onclick: move |_| {
                            let open = *header_profile_open.read();
                            header_profile_open.set(!open);
                        },
                        class: "flex cursor-pointer items-center gap-3 rounded-md py-1 pl-3 transition-colors hover:bg-secondary",
                        div { class: "hidden text-right md:block",
                            div { class: "text-sm font-medium leading-tight text-foreground",
                                "{active_user.full_name.clone().unwrap_or_default()}"
                            }
                            div { class: "text-[10px] font-bold uppercase tracking-wider text-primary",
                                "{active_user.role}"
                            }
                        }
                        div { class: "flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-primary to-primary/60 shadow-inner text-sm font-bold text-foreground",
                            "{active_user.full_name.clone().unwrap_or_default().chars().next().unwrap_or('?').to_uppercase()}"
                        }
                        components::LucideIcon { name: "chevron-down", class: "h-4 w-4 text-muted-foreground" }
                    }

                    // Floating profile menu dropdown
                    if *header_profile_open.read() {
                        div { class: "absolute right-0 top-12 z-50 w-48 rounded-xl border border-border bg-sidebar py-2 shadow-2xl",
                            if !is_client {
                                button {
                                    class: "flex w-full items-center gap-3 px-4 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground border-0 bg-transparent cursor-pointer",
                                    onclick: move |_| {
                                        active_section.set("settings".to_string());
                                        header_profile_open.set(false);
                                    },
                                    components::LucideIcon { name: "settings", class: "h-4 w-4" }
                                    "{t(\"common-settings\", &auth_region.read())}"
                                }
                            }
                            if !is_client {
                                div { class: "my-1 border-t border-border" }
                            }
                            button {
                                class: "flex w-full items-center gap-3 px-4 py-2 text-sm text-red-400 transition-colors hover:bg-red-500/10 hover:text-red-300 border-0 bg-transparent cursor-pointer",
                                onclick: move |_| {
                                    logged_in.set(false);
                                    active_section.set("dashboard".to_string());
                                    header_profile_open.set(false);
                                },
                                components::LucideIcon { name: "logout", class: "h-4 w-4" }
                                "{t(\"common-logout\", &auth_region.read())}"
                            }
                        }
                    }
                }
            }
            if *show_reporting_modal.read() {
                components::Dialog {
                    open: true,
                    title: "Report incident/deviation".to_string(),
                    max_width: "600px".to_string(),
                    onclose: move |_| show_reporting_modal.set(false),
                    div { class: "w-full text-sm",
                        style: "max-width: 500px; padding: 0.5rem;",
                        crate::views::reporting::send::ReportSubmitForm {
                            active_user: active_user.clone(),
                            region: region,
                            report_type: report_type,
                            report_date: report_date,
                            report_subject: report_subject,
                            report_description: report_description,
                            report_is_anonymous: report_is_anonymous,
                            report_tab: report_tab,
                            on_success: move |_| {
                                show_reporting_modal.set(false);
                            }
                        }
                    }
                }
            }
        }
    }
}
