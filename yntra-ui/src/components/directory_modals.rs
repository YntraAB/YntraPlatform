use crate::components;
use dioxus::prelude::*;

#[derive(Clone)]
pub enum DirectoryMember {
    User(yntra_core::WorkspaceUser),
    Client(yntra_core::ClientProfile),
}

impl PartialEq for DirectoryMember {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DirectoryMember::User(a), DirectoryMember::User(b)) => a.id == b.id,
            (DirectoryMember::Client(a), DirectoryMember::Client(b)) => a.id == b.id,
            _ => false,
        }
    }
}

#[derive(Props, Clone)]
pub struct MemberDetailDialogProps {
    pub member: DirectoryMember,
    pub active_user: yntra_core::WorkspaceUser,
    pub onclose: EventHandler<()>,
    pub onedit: EventHandler<DirectoryMember>,
}

impl PartialEq for MemberDetailDialogProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn MemberDetailDialog(props: MemberDetailDialogProps) -> Element {
    let member = props.member.clone();
    let active_user = props.active_user;
    let onclose = props.onclose;
    let onedit = props.onedit;

    let is_admin = active_user.role == "platform_admin" || active_user.role == "admin";

    let (name, role_or_ssn, email, phone, _location_or_address, is_patient) = match &member {
        DirectoryMember::User(u) => {
            let role_name = match u.role.as_str() {
                "platform_admin" => "Platform Administrator",
                "admin" => "Workspace Administrator",
                "assistant" => "Employee",
                "user" => "General User",
                _ => &u.role,
            };
            (
                u.full_name.clone().unwrap_or_else(|| "No Name".to_string()),
                role_name.to_string(),
                Some(u.email.clone()),
                u.phone
                    .clone()
                    .unwrap_or_else(|| "Not specified".to_string()),
                "Not specified".to_string(),
                false,
            )
        }
        DirectoryMember::Client(c) => (
            format!("{} {}", c.first_name, c.last_name),
            c.personal_number
                .clone()
                .unwrap_or_else(|| "Not specified".to_string()),
            None,
            "Not specified".to_string(),
            "Not specified".to_string(),
            true,
        ),
    };

    rsx! {
        components::Dialog {
            open: true,
            title: if is_patient { "Client Profile Details".to_string() } else { "Employee Profile Details".to_string() },
            onclose: move |_| onclose.call(()),
            div { class: "flex flex-col gap-5 w-full text-sm",
                    style: "max-width:400px;",
                // Header Avatar & Name
                div { class: "flex flex-col items-center gap-2 text-center border-b border-border",
                    style: "padding-bottom:1rem;",
                    div { class: "rounded-full text-white flex justify-center items-center font-bold",
                    style: "width:72px; height:72px; background:var(--accent); font-size:1.8rem; box-shadow:0 4px 10px rgba(99,102,241,0.25);",
                        "{name.chars().next().unwrap_or('?')}"
                    }
                    h4 { class: "text-lg font-extrabold text-foreground",
                    style: "margin:0.25rem 0 0 0;", "{name}" }
                    span {
                        style: if is_patient { "font-size:0.75rem; color:var(--text-muted); font-family:monospace;" } else { "font-size:0.75rem; color:var(--accent); font-weight:700; text-transform:uppercase; letter-spacing:0.5px;" },
                        "{role_or_ssn}"
                    }
                }

                // Details List
                div { class: "flex flex-col gap-3.5 bg-white/[0.01] border border-border p-4 rounded-lg",
                    if let Some(em) = email {
                        div { class: "flex flex-col",
                    style: "gap:0.15rem;",
                            span { class: "font-bold text-muted-foreground/60 uppercase",
                    style: "font-size:0.65rem;", "Email Address" }
                            span { class: "text-foreground text-sm",
                    style: "word-break:break-all;", "{em}" }
                        }
                    }

                    div { class: "flex flex-col",
                    style: "gap:0.15rem;",
                        span { class: "font-bold text-muted-foreground/60 uppercase",
                    style: "font-size:0.65rem;", "Phone Contact" }
                        span { class: "text-foreground text-sm", "{phone}" }
                    }

                    if is_patient {
                        if let DirectoryMember::Client(c) = &member {
                            div { class: "flex flex-col",
                    style: "gap:0.15rem;",
                                span { class: "font-bold text-muted-foreground/60 uppercase",
                    style: "font-size:0.65rem;", "Client Classification" }
                                span { class: "text-foreground text-sm", "{c.care_level.clone().unwrap_or_else(|| \"None\".to_string())}" }
                            }
                        }
                    }
                }

                // Footer Actions
                div { class: "flex justify-end gap-2 border-t border-border pt-4 mt-2",
                    button {
                        class: "yntra-btn btn-secondary text-xs",
                        style: "padding:0.5rem 1rem;",
                        onclick: move |_| onclose.call(()),
                        "Close"
                    }
                    if is_admin {
                        button {
                            class: "yntra-btn btn-primary text-xs",
                            style: "padding:0.5rem 1rem;",
                            onclick: move |_| {
                                onedit.call(member.clone());
                            },
                            "Edit Profile"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
pub struct EditMemberDialogProps {
    pub member: DirectoryMember,
    pub onclose: EventHandler<()>,
    pub onsave: EventHandler<()>,
}

impl PartialEq for EditMemberDialogProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn EditMemberDialog(props: EditMemberDialogProps) -> Element {
    let member = props.member.clone();
    let onclose = props.onclose;
    let onsave = props.onsave;

    let is_patient = matches!(&member, DirectoryMember::Client(_));

    let mut is_loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let mut caregiver_name = use_signal(|| match &member {
        DirectoryMember::User(u) => u.full_name.clone().unwrap_or_default(),
        _ => String::new(),
    });
    let mut caregiver_phone = use_signal(|| match &member {
        DirectoryMember::User(u) => u.phone.clone().unwrap_or_default(),
        _ => String::new(),
    });
    let mut caregiver_role = use_signal(|| match &member {
        DirectoryMember::User(u) => u.role.clone(),
        _ => "assistant".to_string(),
    });

    let mut client_first = use_signal(|| match &member {
        DirectoryMember::Client(c) => c.first_name.clone(),
        _ => String::new(),
    });
    let mut client_last = use_signal(|| match &member {
        DirectoryMember::Client(c) => c.last_name.clone(),
        _ => String::new(),
    });
    let mut client_ssn = use_signal(|| match &member {
        DirectoryMember::Client(c) => c.personal_number.clone().unwrap_or_default(),
        _ => String::new(),
    });
    let mut client_level = use_signal(|| match &member {
        DirectoryMember::Client(c) => c
            .care_level
            .clone()
            .unwrap_or_else(|| "High Care".to_string()),
        _ => "High Care".to_string(),
    });

    let state = use_context::<crate::state::AppState>();

    let handle_save = move |_| {
        let member_clone = member.clone();
        spawn(async move {
            is_loading.set(true);
            error_msg.set(None);

            let res = match &member_clone {
                DirectoryMember::User(u) => {
                    let name_val = caregiver_name.read().trim().to_string();
                    let phone_val = caregiver_phone.read().trim().to_string();
                    let role_val = caregiver_role.read().clone();
                    let requester_uid = state.active_user_id.read().clone();

                    yntra_core::update_user_via_directory(
                        requester_uid,
                        u.id.clone(),
                        Some(name_val),
                        Some(phone_val),
                        role_val,
                    )
                    .await
                }
                DirectoryMember::Client(c) => {
                    let first = client_first.read().trim().to_string();
                    let last = client_last.read().trim().to_string();
                    let ssn = client_ssn.read().trim().to_string();
                    let lvl = client_level.read().clone();
                    let requester_uid = state.active_user_id.read().clone();

                    yntra_core::update_client_profile(
                        requester_uid,
                        c.id.clone(),
                        first,
                        last,
                        Some(ssn),
                        Some(lvl),
                    )
                    .await
                }
            };

            match res {
                Ok(_) => {
                    onsave.call(());
                    onclose.call(());
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to save changes: {:?}", e)));
                }
            }
            is_loading.set(false);
        });
    };

    rsx! {
        components::Dialog {
            open: true,
            title: if is_patient { "Edit Client Profile".to_string() } else { "Edit Employee Profile".to_string() },
            onclose: move |_| onclose.call(()),
            div { class: "flex flex-col gap-5 w-full text-sm",
                    style: "max-width:400px;",
                if let Some(err) = error_msg.read().clone() {
                    div { class: "p-3 rounded-lg text-xs",
                    style: "background:rgba(239, 68, 68, 0.1); border:1px solid rgba(239, 68, 68, 0.2); color:var(--danger);",
                        "{err}"
                    }
                }

                if is_patient {
                    div { class: "flex flex-col gap-3.5",
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "First Name" }
                            crate::components::Input {
                                class: "",
                                value: "{client_first}",
                                oninput: move |e: FormEvent| client_first.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "Last Name" }
                            crate::components::Input {
                                class: "",
                                value: "{client_last}",
                                oninput: move |e: FormEvent| client_last.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "Personal Number (SSN)" }
                            crate::components::Input {
                                class: "",
                                value: "{client_ssn}",
                                oninput: move |e: FormEvent| client_ssn.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "Client Classification" }
                            select {
                                class: "yntra-input",
                                value: "{client_level}",
                                onchange: move |e| client_level.set(e.value()),
                                option { value: "High Care", "High Priority" }
                                option { value: "Medium Care", "Medium Priority" }
                                option { value: "Low Care", "Low Priority" }
                            }
                        }
                    }
                } else {
                    div { class: "flex flex-col gap-3.5",
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "Full Name" }
                            crate::components::Input {
                                class: "",
                                value: "{caregiver_name}",
                                oninput: move |e: FormEvent| caregiver_name.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "Phone Number" }
                            crate::components::Input {
                                class: "",
                                value: "{caregiver_phone}",
                                oninput: move |e: FormEvent| caregiver_phone.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground uppercase", "Workspace Role" }
                            select {
                                class: "yntra-input",
                                value: "{caregiver_role}",
                                onchange: move |e| caregiver_role.set(e.value()),
                                option { value: "assistant", "Standard Employee" }
                                option { value: "admin", "Workspace Administrator" }
                                option { value: "user", "General Workspace User" }
                            }
                        }
                    }
                }

                div { class: "flex justify-end gap-2 border-t border-border pt-4 mt-2",
                    button {
                        class: "yntra-btn btn-secondary text-xs",
                        style: "padding:0.6rem 1.2rem;",
                        onclick: move |_| onclose.call(()),
                        "Cancel"
                    }
                    button {
                        class: "yntra-btn btn-primary text-xs",
                        style: "padding:0.6rem 1.2rem;",
                        disabled: *is_loading.read(),
                        onclick: handle_save,
                        if *is_loading.read() { "Saving..." } else { "Save Changes" }
                    }
                }
            }
        }
    }
}
