use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, send_message};

#[derive(Props, Clone, PartialEq)]
pub struct MessageComposeProps {
    pub active_user: WorkspaceUser,
    pub eligible_recipients: Vec<WorkspaceUser>,
    pub messaging_view_tab: Signal<String>,
    pub compose_recipient_id: Signal<Option<String>>,
    pub compose_subject: Signal<String>,
    pub compose_body: Signal<String>,
    pub compose_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub region: String,
}

#[component]
pub fn MessageCompose(props: MessageComposeProps) -> Element {
    let active_user = props.active_user;
    let eligible_recipients = props.eligible_recipients;
    let mut messaging_view_tab = props.messaging_view_tab;
    let mut compose_recipient_id = props.compose_recipient_id;
    let mut compose_subject = props.compose_subject;
    let mut compose_body = props.compose_body;
    let mut compose_status = props.compose_status;
    let region = props.region;

    let mut compose_recipient_open = use_signal(|| false);

    let compose_recipient_label = if let Some(rec_id) = compose_recipient_id.read().clone() {
        if let Some(recipient) = eligible_recipients.iter().find(|r| r.id == rec_id) {
            format!(
                "{} ({})",
                recipient.full_name.clone().unwrap_or_default(),
                recipient.role
            )
        } else {
            t("messages-recipient-placeholder", &region)
        }
    } else {
        t("messages-recipient-placeholder", &region)
    };

    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background duration-300 animate-in fade-in",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-sidebar",
                h2 { class: "flex items-center gap-2.5 text-base font-semibold text-foreground m-0",
                    div { class: "rounded-md bg-primary/10 p-1.5",
                        components::LucideIcon { name: "mail", class: "h-4 w-4 text-primary" }
                    }
                    "{t(\"messages-compose-title\", &region)}"
                }
                button {
                    class: "yntra-btn secondary text-xs flex items-center gap-1.5 h-8 px-3 rounded-lg font-semibold border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer transition-all",
                    onclick: move |_| {
                        messaging_view_tab.set("inbox".to_string());
                    },
                    components::LucideIcon { name: "arrow-left", class: "h-3.5 w-3.5" }
                    "{t(\"messages-cancel\", &region)}"
                }
            }

            div { class: "flex-1 overflow-y-auto px-8 py-6 max-w-2xl flex flex-col gap-5",
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-xs font-bold text-muted-foreground", "{t(\"messages-to\", &region)}" }
                    components::Dropdown {
                        label: compose_recipient_label,
                        open: *compose_recipient_open.read(),
                        ontoggle: move |_| {
                            let cur = *compose_recipient_open.read();
                            compose_recipient_open.set(!cur);
                        },
                        for recipient in eligible_recipients.iter() {
                            {
                                let r_id = recipient.id.clone();
                                let r_name = recipient.full_name.clone().unwrap_or_default();
                                let r_role = recipient.role.clone();
                                let label = format!("{} ({})", r_name, r_role);
                                rsx! {
                                    components::DropdownItem {
                                        key: "{r_id}",
                                        label: label,
                                        onclick: move |_| {
                                            compose_recipient_id.set(Some(r_id.clone()));
                                            compose_recipient_open.set(false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-xs font-bold text-muted-foreground", "{t(\"messages-subject\", &region)}" }
                    input {
                        class: "yntra-input text-sm h-9 px-3 bg-background border border-border rounded-lg text-foreground",
                        value: "{compose_subject}",
                        placeholder: t("messages-subject-placeholder", &region),
                        oninput: move |e| compose_subject.set(e.value()),
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-xs font-bold text-muted-foreground", "{t(\"notes-compose-content-label\", &region)}" }
                    textarea {
                        class: "w-full p-3 border border-border rounded-lg text-foreground text-sm",
                        style: "height:180px; background:rgba(0,0,0,0.2); resize:vertical; line-height:1.5;",
                        value: "{compose_body}",
                        placeholder: t("messages-content-placeholder", &region),
                        oninput: move |e| compose_body.set(e.value()),
                    }
                }
                button {
                    class: "yntra-btn h-9 text-xs font-bold rounded-lg cursor-pointer px-4 self-start mt-2",
                    onclick: move |_| {
                        let rec_opt = compose_recipient_id.read().clone();
                        let sub = compose_subject.read().trim().to_string();
                        let body = compose_body.read().trim().to_string();

                        if let Some(recipient_id) = rec_opt
                            && !sub.is_empty() && !body.is_empty() {
                                compose_status.set("sending".to_string());
                                let workspace_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                let sender_id = active_user.id.clone();
                                let recipient = Some(recipient_id);
                                let requester_id = active_user.id.clone();
                                spawn(async move {
                                    let _ = send_message(
                                        requester_id,
                                        workspace_id,
                                        sender_id,
                                        recipient,
                                        None,
                                        sub,
                                        body,
                                    ).await;
                                });
                                compose_subject.set(String::new());
                                compose_body.set(String::new());
                                compose_status.set("success".to_string());
                                messaging_view_tab.set("sent".to_string());
                            }
                    },
                    if *compose_status.read() == "sending" {
                        "{t(\"messages-sending\", &region)}"
                    } else {
                        "{t(\"messages-send\", &region)}"
                    }
                }
            }
        }
    }
}
