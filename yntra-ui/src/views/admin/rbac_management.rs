use crate::components;
use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct RbacManagementProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub users: Vec<WorkspaceUser>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn RbacManagementView(props: RbacManagementProps) -> Element {
    let mut search_query = use_signal(String::new);
    let mut selected_role_filter = use_signal(|| "all".to_string());

    let mut is_updating = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let active_user_id = props.active_user.id.clone();
    let workspace_id = props.workspace.id.clone();
    let db_trigger = props.db_trigger;

    // Asynchronous resource for role permissions definition
    let permissions_res = use_resource(move || {
        let uid = active_user_id.clone();
        let wsid = workspace_id.clone();
        async move { yntra_core::get_workspace_role_permissions(uid, wsid).await }
    });

    let filtered_users: Vec<WorkspaceUser> = props
        .users
        .iter()
        .filter(|u| {
            let q = search_query.read().to_lowercase();
            let matches_search = q.is_empty()
                || u.email.to_lowercase().contains(&q)
                || u.full_name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&q);

            let rf = selected_role_filter.read().clone();
            let matches_role = rf == "all" || u.role.to_lowercase() == rf;

            matches_search && matches_role
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "p-6 space-y-8 animate-in fade-in duration-300",
            // Section Header
            div { class: "flex items-center justify-between border-b border-border/40 pb-4",
                div { class: "space-y-1",
                    h2 { class: "text-2xl font-extrabold text-foreground tracking-tight flex items-center gap-2",
                        components::LucideIcon { name: "shield-alert", class: "h-6 w-6 text-primary" }
                        "Role-Based Access Control (RBAC)"
                    }
                    p { class: "text-xs text-muted-foreground",
                        "Manage member workspace permissions and inspect role matrix controls."
                    }
                }
                div { class: "inline-flex items-center gap-2 px-3 py-1.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-500 text-xs font-bold",
                    components::LucideIcon { name: "check-circle-2", class: "h-4 w-4" }
                    "Ed25519 Signatures Active"
                }
            }

            // Status Messages
            if let Some(err) = error_msg.read().as_ref() {
                div { class: "p-3.5 rounded-xl bg-red-500/10 border border-red-500/30 text-red-500 text-xs font-medium flex items-center gap-2.5 animate-in fade-in",
                    components::LucideIcon { name: "alert-circle", class: "h-4 w-4 shrink-0" }
                    span { "{err}" }
                }
            }

            if let Some(msg) = status_msg.read().as_ref() {
                div { class: "p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-500 text-xs font-medium flex items-center gap-2.5 animate-in fade-in",
                    components::LucideIcon { name: "check-circle", class: "h-4 w-4 shrink-0" }
                    span { "{msg}" }
                }
            }

            // Workspace Members Table Card
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-4",
                div { class: "flex flex-col md:flex-row md:items-center justify-between gap-4",
                    h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "users", class: "h-4 w-4 text-primary" }
                        "Workspace Members & Assigned Roles ({filtered_users.len()})"
                    }

                    div { class: "flex items-center gap-3",
                        input {
                            class: "px-3.5 py-1.5 rounded-xl border border-input bg-background text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 w-56 transition-all",
                            placeholder: "Search member by name or email...",
                            value: "{search_query.read()}",
                            oninput: move |e| search_query.set(e.value())
                        }
                        select {
                            class: "px-3 py-1.5 rounded-xl border border-input bg-background text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                            value: "{selected_role_filter.read()}",
                            onchange: move |e| selected_role_filter.set(e.value()),
                            option { value: "all", "All Roles" }
                            option { value: "admin", "Administrators" }
                            option { value: "manager", "Managers" }
                            option { value: "member", "Members" }
                            option { value: "viewer", "Viewers" }
                        }
                    }
                }

                div { class: "overflow-x-auto rounded-xl border border-border/30 bg-background",
                    table { class: "w-full text-left border-collapse",
                        thead {
                            tr { class: "border-b border-border/40 bg-muted/40 text-xs font-semibold text-muted-foreground uppercase tracking-wider",
                                th { class: "p-3.5", "Member" }
                                th { class: "p-3.5", "Current Role" }
                                th { class: "p-3.5", "Security Signature" }
                                th { class: "p-3.5 text-right", "Change Role" }
                            }
                        }
                        tbody { class: "divide-y divide-border/20 text-xs text-foreground",
                            for user in filtered_users.iter() {
                                {
                                    let uid = user.id.clone();
                                    let current_role = user.role.clone();
                                    let name_display = user.full_name.clone().unwrap_or_else(|| "User".to_string());
                                    rsx! {
                                        tr { key: "{user.id}", class: "hover:bg-muted/20 transition-colors",
                                            td { class: "p-3.5 font-medium",
                                                div { class: "flex items-center gap-3",
                                                    div { class: "h-8 w-8 rounded-full bg-primary/10 text-primary flex items-center justify-center font-bold text-xs shadow-inner",
                                                        "{name_display.chars().next().unwrap_or('U')}"
                                                    }
                                                    div { class: "flex flex-col",
                                                        span { class: "font-semibold text-foreground text-sm", "{name_display}" }
                                                        span { class: "text-muted-foreground text-xs", "{user.email}" }
                                                    }
                                                }
                                            }
                                            td { class: "p-3.5",
                                                span { class: "px-2.5 py-1 rounded-full text-xs font-bold uppercase tracking-wider bg-primary/10 text-primary border border-primary/20",
                                                    "{user.role}"
                                                }
                                            }
                                            td { class: "p-3.5 font-mono text-xs text-muted-foreground",
                                                if user.public_key.is_some() {
                                                    span { class: "text-emerald-500 font-semibold flex items-center gap-1",
                                                        components::LucideIcon { name: "key-round", class: "h-3 w-3" }
                                                        "Signed"
                                                    }
                                                } else {
                                                    span { class: "text-muted-foreground/70", "Standard Key" }
                                                }
                                            }
                                            td { class: "p-3.5 text-right",
                                                select {
                                                    class: "px-3 py-1 rounded-lg border border-input bg-background text-xs font-medium text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                                                    value: "{current_role}",
                                                    onchange: {
                                                        let req_uid = props.active_user.id.clone();
                                                        let wsid = props.workspace.id.clone();
                                                        let target_uid = user.id.clone();
                                                        let mut trigger = db_trigger;
                                                        move |e: Event<FormData>| {
                                                            let req = req_uid.clone();
                                                            let ws = wsid.clone();
                                                            let target = target_uid.clone();
                                                            let new_r = e.value();
                                                            is_updating.set(true);
                                                            error_msg.set(None);
                                                            status_msg.set(None);
                                                            spawn(async move {
                                                                match yntra_core::update_user_workspace_role(req, ws, target, new_r.clone()).await {
                                                                    Ok(u) => {
                                                                        is_updating.set(false);
                                                                        status_msg.set(Some(format!("Updated role for {} to '{}' (Ed25519 signed)", u.email, new_r)));
                                                                        let trig_val = *trigger.read();
                                                                        trigger.set(trig_val + 1);
                                                                    }
                                                                    Err(err) => {
                                                                        is_updating.set(false);
                                                                        error_msg.set(Some(format!("Failed to update role: {}", err)));
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    option { value: "admin", "Administrator" }
                                                    option { value: "manager", "Operations Manager" }
                                                    option { value: "member", "Standard Member" }
                                                    option { value: "viewer", "Read-Only Viewer" }
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

            // Role Permissions Matrix Card
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-4",
                h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                    components::LucideIcon { name: "grid", class: "h-4 w-4 text-primary" }
                    "Role Capability Matrix"
                }

                match &*permissions_res.read() {
                    Some(Ok(roles)) => rsx! {
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            for role_item in roles.iter() {
                                {
                                    let perms: Vec<String> = serde_json::from_str(&role_item.permissions_json).unwrap_or_default();
                                    rsx! {
                                        div { key: "{role_item.role_id}", class: "p-4 rounded-xl border border-border/40 bg-background space-y-3 shadow-xs",
                                            div { class: "flex items-center justify-between border-b border-border/30 pb-2",
                                                h4 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                                                    components::LucideIcon { name: "shield", class: "h-4 w-4 text-primary" }
                                                    "{role_item.role_name}"
                                                }
                                                span { class: "text-xs font-mono font-bold uppercase text-muted-foreground", "{role_item.role_id}" }
                                            }
                                            p { class: "text-xs text-muted-foreground m-0 leading-relaxed", "{role_item.description}" }

                                            div { class: "flex flex-wrap gap-1.5 pt-1",
                                                for p in perms.iter() {
                                                    span { key: "{p}", class: "px-2 py-0.5 rounded-md bg-muted text-muted-foreground text-xs font-mono font-medium border border-border/40",
                                                        "{p}"
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
                        div { class: "py-6 text-center text-xs text-muted-foreground animate-pulse",
                            "Loading RBAC permission matrix..."
                        }
                    }
                }
            }
        }
    }
}
