use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{MessageItem, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct MessageDetailProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub active_message: MessageItem,
    pub messaging_view_tab: Signal<String>,
    pub active_message_id: Signal<Option<String>>,
    pub compose_recipient_id: Signal<Option<String>>,
    pub compose_subject: Signal<String>,
    pub compose_body: Signal<String>,
    pub region: String,
}

#[component]
pub fn MessageDetail(props: MessageDetailProps) -> Element {
    let active_user = props.active_user;
    let users = props.users;
    let active_message = props.active_message;
    let mut messaging_view_tab = props.messaging_view_tab;
    let mut active_message_id = props.active_message_id;
    let mut compose_recipient_id = props.compose_recipient_id;
    let mut compose_subject = props.compose_subject;
    let mut compose_body = props.compose_body;
    let region = props.region;

    let active_msg_sender_id = active_message.sender_id.clone().unwrap_or_default();
    let active_msg_sender_name = users
        .iter()
        .find(|u| u.id == active_msg_sender_id)
        .and_then(|u| u.full_name.clone())
        .unwrap_or_else(|| "Workspace System".to_string());
    
    let receiver_id = active_message.receiver_id.clone().unwrap_or_default();
    let active_msg_receiver_name = users
        .iter()
        .find(|u| u.id == receiver_id)
        .and_then(|u| u.full_name.clone())
        .unwrap_or_else(|| "Workspace Colleague".to_string());
    
    let active_msg_subject = active_message
        .subject
        .clone()
        .unwrap_or_else(|| "No Subject".to_string());
    
    let active_msg_body = active_message.body.clone().unwrap_or_default();
    let active_msg_created_at = active_message.created_at.clone();

    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background duration-300 animate-in fade-in",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-sidebar",
                div { class: "flex items-center gap-4",
                    button {
                        class: "yntra-btn secondary text-xs flex items-center gap-1.5 h-8 px-3 rounded-lg font-semibold border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer transition-all",
                        onclick: move |_| {
                            active_message_id.set(None);
                        },
                        components::LucideIcon { name: "arrow-left", class: "h-3.5 w-3.5" }
                        "{t(\"common-back\", &region)}"
                    }
                    h2 { class: "text-base font-semibold text-foreground m-0", "{t(\"messages-title\", &region)}" }
                }
                if active_msg_sender_id != active_user.id {
                    button {
                        class: "yntra-btn h-8 text-xs flex items-center gap-1.5 font-bold shadow-md cursor-pointer px-4 rounded-lg",
                        onclick: {
                            let active_msg_sender_id = active_msg_sender_id.clone();
                            let active_msg_subject = active_msg_subject.clone();
                            move |_| {
                                compose_recipient_id.set(Some(active_msg_sender_id.clone()));
                                compose_subject.set(format!("Re: {}", active_msg_subject));
                                compose_body.set(String::new());
                                messaging_view_tab.set("compose".to_string());
                                active_message_id.set(None);
                            }
                        },
                        components::LucideIcon { name: "reply", class: "h-3.5 w-3.5" }
                        "{t(\"messages-reply\", &region)}"
                    }
                }
            }
            
            div { class: "flex-1 overflow-y-auto px-8 py-6 max-w-3xl flex flex-col gap-6",
                div { class: "flex flex-col gap-1 border-b border-border pb-4",
                    div { class: "text-sm text-muted-foreground",
                        span { class: "font-bold", "{t(\"messages-from\", &region)}: " }
                        span { "{active_msg_sender_name}" }
                    }
                    div { class: "text-sm text-muted-foreground",
                        span { class: "font-bold", "{t(\"messages-to\", &region)}: " }
                        span { "{active_msg_receiver_name}" }
                    }
                    div { class: "text-xs text-muted-foreground/60 mt-1",
                        "{t(\"messages-date\", &region)}: {active_msg_created_at}"
                    }
                }
                h3 { class: "text-lg font-bold text-foreground m-0", "{active_msg_subject}" }
                div { class: "bg-white/[0.015] border border-border/40 p-6 rounded-xl text-sm text-foreground",
                    style: "line-height:1.6; white-space:pre-wrap;",
                    "{active_msg_body}"
                }
            }
        }
    }
}
