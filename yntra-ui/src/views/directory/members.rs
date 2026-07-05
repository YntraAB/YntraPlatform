use dioxus::prelude::*;
use crate::components;
use crate::state::AppState;
use super::WorkspaceRole;
use yntra_core::{
    add_client_via_directory, invite_user_via_directory,
    WorkspaceUser, ClientProfile
};

#[derive(Props, Clone)]
pub struct MembersListProps {
    pub active_user: WorkspaceUser,
    pub team_id_unwrap: String,
    pub custom_roles: Vec<WorkspaceRole>,
}

impl PartialEq for MembersListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn MembersList(props: MembersListProps) -> Element {
    let active_user = props.active_user;
    let active_uid_outside = active_user.id.clone();
    let active_uid_for_invite = active_uid_outside.clone();
    let active_uid_for_client = active_uid_outside.clone();
    let team_id_unwrap = props.team_id_unwrap.clone();
    let custom_roles = props.custom_roles.clone();
    let state = use_context::<AppState>();
    
    let workspace_opt = state.workspace.read();
    let workspace = workspace_opt.as_ref().cloned().unwrap_or_else(|| yntra_core::Workspace {
        id: "workspace-1".to_string(),
        name: "Yntra Operations Ltd".to_string(),
        modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
        settings: "{}".to_string(),
        brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
        logo_url: None,
        block_settings: "{}".to_string(),
    });
    let users = state.users.read().clone().unwrap_or_default();
    let _teams = state.teams.read().clone().unwrap_or_default();
    let clients = state.clients.read().clone().unwrap_or_default();
    let mut db_trigger = state.db_trigger;

    // Outer modals visibility signals from context
    let mut show_invite_member_modal = state.show_invite_member_modal;
    let mut show_client_manager_modal = state.show_client_manager_modal;

    // Modals form input signals from context
    let mut new_member_name = state.new_member_name;
    let mut new_member_email = state.new_member_email;
    let mut new_member_role = state.new_member_role;
    let mut new_client_first_name = state.new_client_first_name;
    let mut new_client_last_name = state.new_client_last_name;
    let mut new_client_personal_number = state.new_client_personal_number;
    let mut new_client_care_level = state.new_client_care_level;

    // Local signals for detail modals
    let mut selected_member = use_signal(|| Option::<components::DirectoryMember>::None);
    let mut selected_member_to_edit = use_signal(|| Option::<components::DirectoryMember>::None);



    let _is_admin = active_user.role == "platform_admin" || active_user.role == "admin";

    // Filtering lists
    let team_users: Vec<WorkspaceUser> = users;
    let team_clients: Vec<ClientProfile> = clients
        .iter()
        .filter(|c| c.team_id == Some(team_id_unwrap.clone()))
        .cloned()
        .collect();

    let platform_admins: Vec<WorkspaceUser> = team_users.iter().filter(|u| u.role == "platform_admin").cloned().collect();
    let admins: Vec<WorkspaceUser> = team_users.iter().filter(|u| u.role == "admin").cloned().collect();
    let assistants: Vec<WorkspaceUser> = team_users.iter().filter(|u| u.role != "platform_admin" && u.role != "admin").cloned().collect();
    let first_client = team_clients.first().cloned();



    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-sidebar",
                h2 { class: "flex items-center gap-2 text-base font-medium text-foreground m-0",
                    svg {
                        xmlns: "http://www.w3.org/2000/svg", width: "16", height: "16", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-4 w-4 text-primary",
                        path { d: "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" }
                        circle { cx: "12", cy: "7", r: "4" }
                    }
                    if team_id_unwrap == "all_members" {
                        "Organization Members"
                    } else {
                        "Team Members & Patients"
                    }
                }
            }
            div { class: "scrollbar-dark w-full flex-1 px-8 py-6 box-border",
                if let Some(client) = first_client {
                    div {
                        class: "border border-border bg-white/[0.015] p-6 mb-6 rounded-xl",
                        div {
                            class: "flex flex-col justify-between gap-6 md:flex-row md:items-center",
                            div {
                                class: "flex items-center gap-6",
                                div {
                                    class: "flex h-16 w-16 shrink-0 items-center justify-center rounded-full border-2 border-primary/20 bg-primary/10 text-xl font-bold text-primary",
                                    "{client.first_name.chars().next().unwrap_or('?')}{client.last_name.chars().next().unwrap_or('?')}"
                                }
                                div {
                                    class: "space-y-1",
                                    h3 {
                                        class: "text-xl font-bold tracking-tight text-foreground m-0",
                                        "{client.first_name} {client.last_name}"
                                    }
                                    div {
                                        class: "flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground/60 mt-1",
                                        span { class: "flex items-center gap-1.5",
                                            svg {
                                                xmlns: "http://www.w3.org/2000/svg", width: "14", height: "14", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3.5 w-3.5",
                                                rect { x: "3", y: "4", width: "18", height: "18", rx: "2", ry: "2" }
                                                line { x1: "16", y1: "2", x2: "16", y2: "6" }
                                                line { x1: "8", y1: "2", x2: "8", y2: "6" }
                                                line { x1: "3", y1: "10", x2: "21", y2: "10" }
                                            }
                                            "SSN: {client.personal_number.clone().unwrap_or_default()}"
                                        }
                                        span { class: "flex items-center gap-1.5 font-medium text-primary/80",
                                            svg {
                                                xmlns: "http://www.w3.org/2000/svg", width: "14", height: "14", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3.5 w-3.5",
                                                path { d: "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" }
                                            }
                                            "{client.care_level.clone().unwrap_or_default()}"
                                        }
                                    }
                                }
                            }
                            div {
                                class: "flex items-center gap-2",
                                button {
                                    class: "yntra-btn secondary flex items-center gap-2 border border-border bg-transparent text-xs text-foreground hover:bg-white/[0.04] px-4 py-2 rounded-lg font-semibold cursor-pointer",
                                    onclick: move |_| {
                                        selected_member.set(Some(components::DirectoryMember::Client(client.clone())));
                                    },
                                    svg {
                                        xmlns: "http://www.w3.org/2000/svg", width: "14", height: "14", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3.5 w-3.5 text-primary",
                                        path { d: "M12 20h9" }
                                        path { d: "M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" }
                                    }
                                    span { "Edit Profile" }
                                }
                            }
                        }
                    }
                }

                div { class: "flex flex-col gap-6",
                    if !platform_admins.is_empty() {
                        div { class: "flex flex-col border border-border/40 rounded-xl overflow-hidden bg-white/[0.005]",
                            div { class: "px-6 py-2.5 bg-white/[0.02] border-b border-border/40 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/60",
                                "Platform Admins ({platform_admins.len()})"
                            }
                            for u in platform_admins.clone().into_iter() {
                                div {
                                    key: "{u.id}",
                                    onclick: move |_| {
                                        selected_member.set(Some(components::DirectoryMember::User(u.clone())));
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border/30 px-6 py-3 transition-colors hover:bg-white/[0.02] list-item-hover",
                                    div {
                                        class: "mr-4 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white/[0.04] text-xs font-bold text-primary",
                                        "{u.full_name.clone().unwrap_or_else(|| \"Name Unspecified\".to_string()).chars().next().unwrap_or('?')}"
                                    }
                                    div {
                                        class: "w-64 shrink-0 pr-4 text-sm font-semibold text-foreground md:w-80",
                                        "{u.full_name.clone().unwrap_or_else(|| \"Name Unspecified\".to_string())}"
                                        div { class: "mt-0.5 text-xs font-normal text-muted-foreground/60 truncate", "{u.email}" }
                                    }
                                    div { class: "min-w-0 flex-1 pr-4" }
                                    components::LucideIcon { name: "chevron-right", class: "h-5 w-5 text-muted-foreground/40" }
                                }
                            }
                        }
                    }

                    if !admins.is_empty() {
                        div { class: "flex flex-col border border-border/40 rounded-xl overflow-hidden bg-white/[0.005]",
                            div { class: "px-6 py-2.5 bg-white/[0.02] border-b border-border/40 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/60",
                                "Administrators ({admins.len()})"
                            }
                            for u in admins.clone().into_iter() {
                                div {
                                    key: "{u.id}",
                                    onclick: move |_| {
                                        selected_member.set(Some(components::DirectoryMember::User(u.clone())));
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border/30 px-6 py-3 transition-colors hover:bg-white/[0.02] list-item-hover",
                                    div {
                                        class: "mr-4 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white/[0.04] text-xs font-bold text-primary",
                                        "{u.full_name.clone().unwrap_or_else(|| \"Name Unspecified\".to_string()).chars().next().unwrap_or('?')}"
                                    }
                                    div {
                                        class: "w-64 shrink-0 pr-4 text-sm font-semibold text-foreground md:w-80",
                                        "{u.full_name.clone().unwrap_or_else(|| \"Name Unspecified\".to_string())}"
                                        div { class: "mt-0.5 text-xs font-normal text-muted-foreground/60 truncate", "{u.email}" }
                                    }
                                    div { class: "min-w-0 flex-1 pr-4" }
                                    components::LucideIcon { name: "chevron-right", class: "h-5 w-5 text-muted-foreground/40" }
                                }
                            }
                        }
                    }

                    if !assistants.is_empty() {
                        div { class: "flex flex-col border border-border/40 rounded-xl overflow-hidden bg-white/[0.005]",
                            div { class: "px-6 py-2.5 bg-white/[0.02] border-b border-border/40 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/60",
                                "Employees & Staff ({assistants.len()})"
                            }
                            for u in assistants.clone().into_iter() {
                                div {
                                    key: "{u.id}",
                                    onclick: move |_| {
                                        selected_member.set(Some(components::DirectoryMember::User(u.clone())));
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border/30 px-6 py-3 transition-colors hover:bg-white/[0.02] list-item-hover",
                                    div {
                                        class: "mr-4 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white/[0.04] text-xs font-bold text-primary",
                                        "{u.full_name.clone().unwrap_or_else(|| \"Name Unspecified\".to_string()).chars().next().unwrap_or('?')}"
                                    }
                                    div {
                                        class: "w-64 shrink-0 pr-4 text-sm font-semibold text-foreground md:w-80",
                                        "{u.full_name.clone().unwrap_or_else(|| \"Name Unspecified\".to_string())}"
                                        div { class: "mt-0.5 text-xs font-normal text-muted-foreground/60 truncate", "{u.email}" }
                                    }
                                    div { class: "min-w-0 flex-1 pr-4" }
                                    components::LucideIcon { name: "chevron-right", class: "h-5 w-5 text-muted-foreground/40" }
                                }
                            }
                        }
                    }
                }
            }
        }



        // Invite Caregiver Modal
        {
            let ws_id = workspace.id.clone();
            rsx! {
                components::Dialog {
                    open: *show_invite_member_modal.read(),
                    title: "Invite Employee / Admin".to_string(),
                    onclose: move |_| show_invite_member_modal.set(false),
                    div { class: "flex flex-col gap-5",
                        div {
                            label { class: "text-xs font-bold text-muted-foreground",
                                "Full Name"
                            }
                            input {
                                class: "yntra-input",
                                value: "{new_member_name}",
                                placeholder: "e.g. Marie Andersson",
                                oninput: move |e| new_member_name.set(e.value()),
                            }
                        }
                        div {
                            label { class: "text-xs font-bold text-muted-foreground",
                                "Email Address"
                            }
                            input {
                                class: "yntra-input",
                                value: "{new_member_email}",
                                placeholder: "marie@yntra.se",
                                oninput: move |e| new_member_email.set(e.value()),
                            }
                        }
                        div {
                            label { class: "text-xs font-bold text-muted-foreground",
                                "Workspace Role"
                            }
                            select {
                                class: "yntra-input",
                                value: "{new_member_role}",
                                onchange: move |e| new_member_role.set(e.value()),
                                option { value: "assistant", "Standard Employee" }
                                for custom_r in custom_roles.iter() {
                                    option { value: "{custom_r.name}", "{custom_r.name} (Custom Role)" }
                                }
                                option { value: "admin", "Workspace Administrator" }
                                option { value: "user", "General Workspace User" }
                            }
                        }
                        button {
                            class: "yntra-btn",
                            onclick: move |_| {
                                let email = new_member_email.read().trim().to_string();
                                let name = new_member_name.read().trim().to_string();
                                if !email.is_empty() && !name.is_empty() {
                                    let w_id = ws_id.clone();
                                    let role = new_member_role.read().clone();
                                    let active_uid = active_uid_for_invite.clone();
                                    spawn(async move {
                                        let _ = invite_user_via_directory(
                                            active_uid,
                                            w_id,
                                            email,
                                            name,
                                            role,
                                        ).await;
                                    });
                                    new_member_name.set(String::new());
                                    new_member_email.set(String::new());
                                    show_invite_member_modal.set(false);
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                }
                            },
                            "Send Invite"
                        }
                    }
                }
            }
        }

        // Register Patient Modal
        {
            let ws_id = workspace.id.clone();
            let tid = team_id_unwrap.clone();
            rsx! {
                components::Dialog {
                    open: *show_client_manager_modal.read(),
                    title: "Register New Client".to_string(),
                    onclose: move |_| show_client_manager_modal.set(false),
                    div { class: "flex flex-col gap-5",
                        div { class: "grid gap-4",
                        style: "grid-template-columns: 1fr 1fr;",
                            div {
                                label { class: "text-xs font-bold text-muted-foreground",
                                    "First Name"
                                }
                                input {
                                    class: "yntra-input",
                                    value: "{new_client_first_name}",
                                    placeholder: "Lars",
                                    oninput: move |e| new_client_first_name.set(e.value()),
                                }
                            }
                            div {
                                label { class: "text-xs font-bold text-muted-foreground",
                                    "Last Name"
                                }
                                input {
                                    class: "yntra-input",
                                    value: "{new_client_last_name}",
                                    placeholder: "Johansson",
                                    oninput: move |e| new_client_last_name.set(e.value()),
                                }
                            }
                        }
                        div {
                            label { class: "text-xs font-bold text-muted-foreground",
                                "Personal Number (SSN)"
                            }
                            input {
                                class: "yntra-input",
                                value: "{new_client_personal_number}",
                                placeholder: "19481105-4321",
                                oninput: move |e| new_client_personal_number.set(e.value()),
                            }
                        }
                        div {
                             label { class: "text-xs font-bold text-muted-foreground",
                                 "Client Classification"
                             }
                            select {
                                class: "yntra-input",
                                value: "{new_client_care_level}",
                                onchange: move |e| new_client_care_level.set(e.value()),
                                 option { value: "High Care", "High Priority" }
                                 option { value: "Medium Care", "Medium Priority" }
                                 option { value: "Low Care", "Low Priority" }
                            }
                        }
                        button {
                            class: "yntra-btn",
                            onclick: move |_| {
                                let fname = new_client_first_name.read().trim().to_string();
                                let lname = new_client_last_name.read().trim().to_string();
                                let ssn = new_client_personal_number.read().trim().to_string();
                                if !fname.is_empty() && !lname.is_empty() {
                                    let w_id = ws_id.clone();
                                    let t_id = Some(tid.clone());
                                    let care_lvl = new_client_care_level.read().clone();
                                    let requester_uid = active_uid_for_client.clone();
                                    spawn(async move {
                                        let _ = add_client_via_directory(
                                            requester_uid,
                                            w_id,
                                            t_id,
                                            fname,
                                            lname,
                                            ssn,
                                            care_lvl,
                                        ).await;
                                    });
                                    new_client_first_name.set(String::new());
                                    new_client_last_name.set(String::new());
                                    new_client_personal_number.set(String::new());
                                    show_client_manager_modal.set(false);
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                }
                            },
                            "Register Client Profile"
                        }
                    }
                }
            }
        }

        // Details Modals
        if let Some(m) = selected_member.read().clone() {
            components::MemberDetailDialog {
                member: m.clone(),
                active_user: active_user.clone(),
                onclose: move |_| selected_member.set(None),
                onedit: move |m_edit| {
                    selected_member.set(None);
                    selected_member_to_edit.set(Some(m_edit));
                }
            }
        }
        if let Some(m) = selected_member_to_edit.read().clone() {
            components::EditMemberDialog {
                member: m.clone(),
                onclose: move |_| selected_member_to_edit.set(None),
                onsave: move |_| {
                    let current = *db_trigger.read();
                    db_trigger.set(current + 1);
                }
            }
        }
    }
}
