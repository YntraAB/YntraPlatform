use dioxus::prelude::*;
use yntra_core::MessageItem;
use yntra_core::WorkspaceUser;
use yntra_core::mark_message_read;
use crate::components;
use crate::locales::t;

mod compose;
mod detail;

#[derive(Props, Clone)]
pub struct MessagingViewProps {
    pub active_user: WorkspaceUser,
    pub unread_messages_count: usize,
    pub messaging_view_tab: Signal<String>,
    pub active_message_id: Signal<Option<String>>,
    pub compose_recipient_id: Signal<Option<String>>,
    pub compose_subject: Signal<String>,
    pub compose_body: Signal<String>,
    pub compose_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub is_client: bool,
}

impl PartialEq for MessagingViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn MessagingView(props: MessagingViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_user = props.active_user;
    let users = state.users.read().clone().unwrap_or_default();
    let messages = state.messages.read().clone().unwrap_or_default();

    let mut messaging_view_tab = props.messaging_view_tab;
    let mut active_message_id = props.active_message_id;
    let compose_recipient_id = props.compose_recipient_id;
    let mut compose_subject = props.compose_subject;
    let mut compose_body = props.compose_body;
    let mut compose_status = props.compose_status;
    let db_trigger = props.db_trigger;

    let current_tab = messaging_view_tab.read().clone();
    let mut filtered_messages: Vec<MessageItem> = messages // Newest first
        .iter()
        .filter(|m| {
            if current_tab == "inbox" {
                m.receiver_id == Some(active_user.id.clone()) || m.target_team_id.is_some()
            } else if current_tab == "sent" {
                m.sender_id == Some(active_user.id.clone())
            } else {
                false
            }
        })
        .cloned()
        .collect();
    filtered_messages.reverse(); // Left folders & message list pane

    let active_msg = active_message_id
        .read()
        .clone()
        .and_then(|id| messages.iter().find(|m| m.id == id).cloned());

    let eligible_recipients: Vec<WorkspaceUser> = users
        .iter()
        .filter(|u| u.id != active_user.id)
        .cloned()
        .collect();

    let user_prefs: serde_json::Value = serde_json::from_str(&active_user.preferences).unwrap_or_default();
    let region = user_prefs.get("language").and_then(|l| l.as_str()).unwrap_or("US").to_string();

    let users_for_messaging = users.clone();

    rsx! {
        if current_tab == "compose" {
            compose::MessageCompose {
                active_user: active_user.clone(),
                eligible_recipients: eligible_recipients,
                messaging_view_tab: messaging_view_tab,
                compose_recipient_id: compose_recipient_id,
                compose_subject: compose_subject,
                compose_body: compose_body,
                compose_status: compose_status,
                db_trigger: db_trigger,
                region: region,
            }
        } else if let Some(msg) = active_msg {
            detail::MessageDetail {
                active_user: active_user.clone(),
                users: users,
                active_message: msg,
                messaging_view_tab: messaging_view_tab,
                active_message_id: active_message_id,
                compose_recipient_id: compose_recipient_id,
                compose_subject: compose_subject,
                compose_body: compose_body,
                region: region,
            }
        } else {
            // Messages List View Full Width
            div { class: "relative flex h-full flex-1 flex-col bg-background duration-300 animate-in fade-in",
                div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-sidebar",
                    h2 { class: "flex items-center gap-2.5 text-base font-semibold text-foreground m-0",
                        div { class: "rounded-md bg-primary/10 p-1.5",
                            components::LucideIcon { name: "mail", class: "h-4 w-4 text-primary" }
                        }
                        {
                            if current_tab == "inbox" {
                                t("messages-inbox", &region)
                            } else {
                                t("messages-sent", &region)
                            }
                        }
                    }
                    
                    div { class: "flex items-center gap-4",
                        components::Tabs {
                            tabs: vec![
                                components::tabs::TabItem {
                                    value: "inbox".to_string(),
                                    label: t("messages-inbox", &region),
                                    icon: Some("inbox".to_string()),
                                },
                                components::tabs::TabItem {
                                    value: "sent".to_string(),
                                    label: t("messages-sent", &region),
                                    icon: Some("send".to_string()),
                                },
                            ],
                            active_tab: current_tab.clone(),
                            onchange: move |val| {
                                messaging_view_tab.set(val);
                                active_message_id.set(None);
                            },
                        }
                    }
                }
                
                div { class: "scrollbar-dark w-full flex-1 overflow-y-auto flex flex-col",
                    if filtered_messages.is_empty() {
                        div { class: "text-center text-muted-foreground/60 py-16",
                            components::LucideIcon { name: "mail", class: "h-12 w-12 mx-auto mb-3 opacity-20" }
                            p { class: "text-sm m-0", "{t(\"messages-empty-state\", &region)}" }
                        }
                    } else {
                        {
                            let buffer_sig = use_signal(|| 5_usize);
                            let msgs = filtered_messages.clone();
                            let active_u = active_user.clone();
                            let cur_tab = current_tab.clone();
                            let reg = region.clone();
                            let user_names: std::collections::HashMap<String, String> = users_for_messaging
                                .iter()
                                .map(|u| (u.id.clone(), u.full_name.clone().unwrap_or_default()))
                                .collect();
                            rsx! {
                                components::VirtualList {
                                    count: msgs.len(),
                                    buffer: buffer_sig,
                                    estimate_size: move |_| 72_u32,
                                    render_item: move |idx: usize| {
                                        let msg = &msgs[idx];
                                        let msg_id = msg.id.clone();
                                        let is_unread = !msg.is_read && msg.receiver_id == Some(active_u.id.clone());

                                        let display_name = if cur_tab == "inbox" {
                                            let sender_id = msg.sender_id.clone().unwrap_or_default();
                                            user_names.get(&sender_id).cloned().filter(|name| !name.is_empty())
                                                .unwrap_or_else(|| t("messages-system", &reg))
                                        } else {
                                            let receiver_id = msg.receiver_id.clone().unwrap_or_default();
                                            user_names.get(&receiver_id).cloned().filter(|name| !name.is_empty())
                                                .unwrap_or_else(|| t("messages-person", &reg))
                                        };
                                        let subject_str = msg
                                            .subject
                                            .clone()
                                            .unwrap_or_else(|| t("messages-no-header", &reg));
                                        let snippet = msg.body.clone().unwrap_or_default();
                                        let snippet_truncated = if snippet.len() > 80 {
                                            format!("{}...", &snippet[0..80])
                                        } else {
                                            snippet
                                        };
                                        
                                        let icon_name = if is_unread { "mail" } else { "mail-open" };
                                        let class_sender = if is_unread { "font-bold text-foreground" } else { "font-semibold text-foreground/80" };
                                        let class_subject = if is_unread { "font-bold text-foreground" } else { "text-foreground/90" };

                                        let requester_id = active_u.id.clone();

                                        rsx! {
                                            div {
                                                key: "{msg_id}",
                                                onclick: move |_| {
                                                    active_message_id.set(Some(msg_id.clone()));
                                                    if is_unread {
                                                        let m_id = msg_id.clone();
                                                        let r_id = requester_id.clone();
                                                        spawn(async move {
                                                            let _ = mark_message_read(r_id, m_id).await;
                                                        });
                                                    }
                                                },
                                                class: "group flex items-center border-b border-border/30 px-8 py-4 transition-colors hover:bg-white/[0.015] list-item-hover cursor-pointer",
                                                
                                                div { class: "mr-4 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white/[0.04] text-primary transition-colors group-hover:bg-primary/10",
                                                    components::LucideIcon { name: icon_name, class: "h-4 w-4 text-primary" }
                                                }
                                                
                                                div { class: "w-48 shrink-0 truncate pr-4 text-sm font-semibold text-foreground md:w-64",
                                                    span { class: "{class_sender}",
                                                        "{display_name}"
                                                    }
                                                    if is_unread {
                                                        span { class: "ml-2 h-1.5 w-1.5 rounded-full bg-primary inline-block" }
                                                    }
                                                }
                                                
                                                div { class: "flex min-w-0 flex-1 items-center gap-2 truncate pr-4 text-sm",
                                                    span { class: "{class_subject}", "{subject_str}" }
                                                    span { class: "truncate text-muted-foreground/60 font-light", "- {snippet_truncated}" }
                                                }
                                                
                                                div { class: "w-32 shrink-0 text-right text-xs font-mono text-muted-foreground/60 pr-2",
                                                    "{msg.created_at}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Floating Action Button
                div { class: "absolute bottom-10 right-10 z-20",
                    button {
                        class: "yntra-btn flex h-14 pl-5 pr-7 bg-foreground text-background hover:bg-foreground/90 shadow-[0_20px_50px_rgba(0,0,0,0.3)] font-bold items-center gap-2.5 transition-all hover:scale-105 active:scale-95 rounded-full cursor-pointer border-0",
                        onclick: move |_| {
                            messaging_view_tab.set("compose".to_string());
                            active_message_id.set(None);
                            compose_status.set("idle".to_string());
                            compose_subject.set(String::new());
                            compose_body.set(String::new());
                        },
                        components::LucideIcon { name: "plus", class: "h-5 w-5 text-background" }
                        span { "{t(\"messages-new-message\", &region)}" }
                    }
                }
            }
        }
    }
}
