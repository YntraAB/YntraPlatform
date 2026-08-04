use crate::components;
use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct TeamInvitationsProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn TeamInvitationsView(props: TeamInvitationsProps) -> Element {
    let mut invite_email = use_signal(String::new);
    let mut invite_name = use_signal(String::new);
    let mut invite_role = use_signal(|| "member".to_string());

    let mut is_creating = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut copied_code = use_signal(|| Option::<String>::None);

    let active_user_id = props.active_user.id.clone();
    let workspace_id = props.workspace.id.clone();
    let mut db_trigger = props.db_trigger;

    // Asynchronous resource for invitations list
    let invitations_res = use_resource(move || {
        let uid = active_user_id.clone();
        let wsid = workspace_id.clone();
        let _trig = *db_trigger.read();
        async move { yntra_core::get_workspace_invitations(uid, wsid).await }
    });

    let active_uid_create = props.active_user.id.clone();
    let workspace_id_create = props.workspace.id.clone();

    let handle_create_invite = move |_| {
        let email_val = invite_email.read().trim().to_string();
        let name_val = invite_name.read().trim().to_string();
        let role_val = invite_role.read().clone();

        if email_val.is_empty() || !email_val.contains('@') {
            error_msg.set(Some("Please enter a valid email address.".to_string()));
            return;
        }

        is_creating.set(true);
        error_msg.set(None);
        status_msg.set(None);

        let uid = active_uid_create.clone();
        let wsid = workspace_id_create.clone();
        let mut trigger = db_trigger;

        spawn(async move {
            let res = yntra_core::create_workspace_invitation(
                uid.clone(),
                wsid,
                email_val,
                if name_val.is_empty() {
                    "Invited User".to_string()
                } else {
                    name_val
                },
                role_val,
            )
            .await;

            is_creating.set(false);
            match res {
                Ok(inv) => {
                    invite_email.set(String::new());
                    invite_name.set(String::new());
                    status_msg.set(Some(format!(
                        "Invitation code {} successfully generated!",
                        inv.code
                    )));
                    let trig_val = *trigger.read();
                    trigger.set(trig_val + 1);
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to create invitation: {}", e)));
                }
            }
        });
    };

    rsx! {
        div { class: "p-6 space-y-8 animate-in fade-in duration-300",
            // Section Header
            div { class: "flex items-center justify-between border-b border-border/40 pb-4",
                div { class: "space-y-1",
                    h2 { class: "text-2xl font-extrabold text-foreground tracking-tight flex items-center gap-2",
                        components::LucideIcon { name: "user-plus", class: "h-6 w-6 text-primary" }
                        "Team Invitation Manager"
                    }
                    p { class: "text-xs text-muted-foreground",
                        "Issue secure invitation links and email invites to join workspace '"
                        span { class: "font-semibold text-foreground", "{props.workspace.name}" }
                        "'"
                    }
                }
            }

            // Create Invitation Form Card
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-6",
                h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                    components::LucideIcon { name: "mail-plus", class: "h-4 w-4 text-primary" }
                    "Generate New Invitation"
                }

                if let Some(err) = error_msg.read().as_ref() {
                    div { class: "p-3.5 rounded-xl bg-red-500/10 border border-red-500/30 text-red-500 text-xs font-medium flex items-center gap-2.5",
                        components::LucideIcon { name: "alert-circle", class: "h-4 w-4 shrink-0" }
                        span { "{err}" }
                    }
                }

                if let Some(msg) = status_msg.read().as_ref() {
                    div { class: "p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-500 text-xs font-medium flex items-center gap-2.5",
                        components::LucideIcon { name: "check-circle", class: "h-4 w-4 shrink-0" }
                        span { "{msg}" }
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4 items-end",
                    div { class: "space-y-1.5",
                        label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Member Email *" }
                        input {
                            class: "w-full px-4 py-2 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                            placeholder: "colleague@domain.com",
                            value: "{invite_email.read()}",
                            oninput: move |e| invite_email.set(e.value())
                        }
                    }

                    div { class: "space-y-1.5",
                        label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Full Name (Optional)" }
                        input {
                            class: "w-full px-4 py-2 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                            placeholder: "Jane Doe",
                            value: "{invite_name.read()}",
                            oninput: move |e| invite_name.set(e.value())
                        }
                    }

                    div { class: "space-y-1.5",
                        label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "RBAC Role" }
                        select {
                            class: "w-full px-3 py-2 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                            value: "{invite_role.read()}",
                            onchange: move |e| invite_role.set(e.value()),
                            option { value: "admin", "Administrator" }
                            option { value: "manager", "Operations Manager" }
                            option { value: "member", "Standard Member" }
                            option { value: "viewer", "Read-Only Viewer" }
                        }
                    }
                }

                div { class: "flex justify-end pt-2",
                    button {
                        class: "px-6 py-2.5 rounded-xl bg-primary text-primary-foreground font-bold text-sm hover:opacity-90 transition-all flex items-center gap-2 shadow-sm disabled:opacity-50",
                        disabled: *is_creating.read(),
                        onclick: handle_create_invite,
                        if *is_creating.read() {
                            components::LucideIcon { name: "loader-2", class: "h-4 w-4 animate-spin" }
                            span { "Generating..." }
                        } else {
                            components::LucideIcon { name: "send", class: "h-4 w-4" }
                            span { "Generate & Send Invitation" }
                        }
                    }
                }
            }

            // Pending & Active Invitations List
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-4",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "list", class: "h-4 w-4 text-primary" }
                        "Workspace Invitations"
                    }
                    button {
                        class: "p-2 rounded-xl border border-input bg-background text-xs font-medium hover:bg-accent transition-colors flex items-center gap-1.5",
                        onclick: move |_| {
                            let val = *db_trigger.read();
                            db_trigger.set(val + 1);
                        },
                        components::LucideIcon { name: "refresh-cw", class: "h-3.5 w-3.5" }
                        "Refresh"
                    }
                }

                match &*invitations_res.read() {
                    Some(Ok(list)) if !list.is_empty() => rsx! {
                        div { class: "overflow-x-auto rounded-xl border border-border/30 bg-background",
                            table { class: "w-full text-left border-collapse",
                               thead {
                                   tr { class: "border-b border-border/40 bg-muted/40 text-xs font-semibold text-muted-foreground uppercase tracking-wider",
                                       th { class: "p-3.5", "Recipient" }
                                       th { class: "p-3.5", "Role" }
                                       th { class: "p-3.5", "Invite Code" }
                                       th { class: "p-3.5", "Status" }
                                       th { class: "p-3.5 text-right", "Actions" }
                                   }
                               }
                               tbody { class: "divide-y divide-border/20 text-xs text-foreground",
                                   for inv in list.iter() {
                                       {
                                           let code = inv.code.clone();
                                           let is_copied = copied_code.read().as_deref() == Some(&code);
                                           rsx! {
                                               tr { key: "{inv.code}", class: "hover:bg-muted/20 transition-colors",
                                                   td { class: "p-3.5 font-medium",
                                                       div { class: "flex flex-col",
                                                           span { class: "font-semibold text-foreground text-sm", "{inv.full_name}" }
                                                           span { class: "text-muted-foreground text-xs", "{inv.email}" }
                                                       }
                                                   }
                                                   td { class: "p-3.5",
                                                       span { class: "px-2.5 py-1 rounded-full text-xs font-bold uppercase tracking-wider bg-primary/10 text-primary border border-primary/20",
                                                           "{inv.role}"
                                                       }
                                                   }
                                                   td { class: "p-3.5 font-mono text-muted-foreground", "{inv.code}" }
                                                   td { class: "p-3.5",
                                                       if inv.activated {
                                                           span { class: "inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-500 border border-emerald-500/20",
                                                               components::LucideIcon { name: "check-circle-2", class: "h-3 w-3" }
                                                               "Activated"
                                                           }
                                                       } else {
                                                           span { class: "inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-amber-500/10 text-amber-500 border border-amber-500/20 animate-pulse",
                                                               components::LucideIcon { name: "clock", class: "h-3 w-3" }
                                                               "Pending"
                                                           }
                                                       }
                                                   }
                                                   td { class: "p-3.5 text-right space-x-2",
                                                       button {
                                                           class: format!(
                                                               "px-3 py-1.5 rounded-lg border text-xs font-medium transition-all inline-flex items-center gap-1.5 {}",
                                                               if is_copied {
                                                                   "border-emerald-500/40 bg-emerald-500/10 text-emerald-500"
                                                               } else {
                                                                   "border-input bg-background hover:bg-accent text-foreground"
                                                               }
                                                           ),
                                                           onclick: {
                                                               let c = code.clone();
                                                               move |_| {
                                                                   copied_code.set(Some(c.clone()));
                                                               }
                                                           },
                                                           components::LucideIcon { name: if is_copied { "check" } else { "copy" }, class: "h-3.5 w-3.5" }
                                                           { if is_copied { "Copied!" } else { "Copy Link" } }
                                                       }
                                                       if !inv.activated {
                                                           button {
                                                               class: "px-3 py-1.5 rounded-lg border border-red-500/30 text-red-500 bg-red-500/5 hover:bg-red-500/15 text-xs font-medium transition-colors inline-flex items-center gap-1",
                                                               onclick: {
                                                                   let req_uid = props.active_user.id.clone();
                                                                   let wsid = props.workspace.id.clone();
                                                                   let code_to_revoke = inv.code.clone();
                                                                   let mut trigger = db_trigger;
                                                                   move |_| {
                                                                       let u = req_uid.clone();
                                                                       let w = wsid.clone();
                                                                       let c = code_to_revoke.clone();
                                                                       spawn(async move {
                                                                           if let Ok(_) = yntra_core::revoke_workspace_invitation(u, w, c).await {
                                                                               let trig_val = *trigger.read();
                                                                               trigger.set(trig_val + 1);
                                                                           }
                                                                       });
                                                                   }
                                                               },
                                                               components::LucideIcon { name: "x-circle", class: "h-3.5 w-3.5" }
                                                               "Revoke"
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
                    Some(Ok(_)) => rsx! {
                        div { class: "py-12 text-center border border-dashed border-border/60 rounded-xl bg-muted/20 text-muted-foreground space-y-2",
                            components::LucideIcon { name: "mail-question", class: "h-8 w-8 mx-auto text-muted-foreground/60" }
                            p { class: "text-sm font-medium m-0", "No active invitation links found" }
                            p { class: "text-xs text-muted-foreground m-0", "Use the form above to issue team invitations." }
                        }
                    },
                    Some(Err(err)) => rsx! {
                        div { class: "p-4 rounded-xl bg-red-500/10 text-red-500 text-xs font-medium",
                            "Failed to load invitations: {err}"
                        }
                    },
                    None => rsx! {
                        div { class: "py-8 text-center text-muted-foreground text-xs animate-pulse",
                            "Loading workspace invitations..."
                        }
                    }
                }
            }
        }
    }
}
