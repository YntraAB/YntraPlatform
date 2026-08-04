use dioxus::prelude::*;
use yntra_core::InAppNotification;

#[derive(Props, Clone, PartialEq)]
pub struct NotificationBellProps {
    pub notifications: Vec<InAppNotification>,
    pub on_mark_read: EventHandler<String>,
    pub on_mark_all_read: EventHandler<()>,
}

#[component]
pub fn NotificationBell(props: NotificationBellProps) -> Element {
    let mut is_open = use_signal(|| false);
    let mut filter_unread_only = use_signal(|| false);

    let unread_count = props.notifications.iter().filter(|n| !n.is_read).count();
    
    let filtered_notifications: Vec<InAppNotification> = if *filter_unread_only.read() {
        props.notifications.iter().filter(|n| !n.is_read).cloned().collect()
    } else {
        props.notifications.clone()
    };

    rsx! {
        div {
            class: "relative inline-block text-left",
            
            // Bell Button
            button {
                r#type: "button",
                class: "relative p-2 text-slate-400 hover:text-white rounded-lg hover:bg-slate-800/60 transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500/50",
                onclick: move |_| is_open.toggle(),
                title: "Notifications",
                
                // Bell SVG Icon
                svg {
                    class: "w-5 h-5",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                        d: "M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
                    }
                }

                // Unread Badge Counter
                if unread_count > 0 {
                    span {
                        class: "absolute top-1 right-1 flex h-4 min-w-[16px] px-1 items-center justify-center rounded-full bg-rose-500 text-[10px] font-bold text-white shadow-sm ring-2 ring-slate-900 animate-pulse",
                        if unread_count > 99 { "99+" } else { "{unread_count}" }
                    }
                }
            }

            // Notification Popover Dropdown
            if *is_open.read() {
                div {
                    class: "absolute right-0 mt-2 w-80 md:w-96 rounded-xl bg-slate-900 border border-slate-800 shadow-2xl z-50 overflow-hidden backdrop-blur-xl animate-in fade-in slide-in-from-top-2 duration-150",
                    
                    // Header
                    div {
                        class: "flex items-center justify-between px-4 py-3 border-b border-slate-800/80 bg-slate-900/90",
                        div { class: "flex items-center gap-2",
                            span { class: "font-semibold text-sm text-slate-100", "Notifications" }
                            if unread_count > 0 {
                                span { class: "px-2 py-0.5 text-xs bg-indigo-500/20 text-indigo-400 font-medium rounded-full border border-indigo-500/30", "{unread_count} new" }
                            }
                        }
                        div { class: "flex items-center gap-2 text-xs",
                            button {
                                class: "text-slate-400 hover:text-indigo-400 transition-colors font-medium",
                                onclick: move |_| props.on_mark_all_read.call(()),
                                "Mark all as read"
                            }
                        }
                    }

                    // Filter Tabs
                    div {
                        class: "flex items-center px-4 py-2 bg-slate-950/40 border-b border-slate-800/50 gap-2 text-xs",
                        button {
                            class: if !*filter_unread_only.read() { "px-2.5 py-1 rounded-md bg-slate-800 text-slate-100 font-medium" } else { "px-2.5 py-1 rounded-md text-slate-400 hover:text-slate-200" },
                            onclick: move |_| filter_unread_only.set(false),
                            "All"
                        }
                        button {
                            class: if *filter_unread_only.read() { "px-2.5 py-1 rounded-md bg-slate-800 text-slate-100 font-medium" } else { "px-2.5 py-1 rounded-md text-slate-400 hover:text-slate-200" },
                            onclick: move |_| filter_unread_only.set(true),
                            "Unread"
                        }
                    }

                    // Notification Item List
                    div {
                        class: "max-h-80 overflow-y-auto divide-y divide-slate-800/40",
                        
                        if filtered_notifications.is_empty() {
                            div {
                                class: "p-8 text-center text-slate-500 text-xs flex flex-col items-center gap-2",
                                svg { class: "w-8 h-8 text-slate-600 opacity-60", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" }
                                }
                                "No notifications to show"
                            }
                        } else {
                            for item in filtered_notifications {
                                {
                                    let notif_id = item.id.clone();
                                    let is_unread = !item.is_read;
                                    let bg_style = if is_unread { "bg-indigo-950/20 hover:bg-indigo-900/30" } else { "bg-transparent hover:bg-slate-800/40" };
                                    
                                    rsx! {
                                        div {
                                            key: "{item.id}",
                                            class: "p-3 flex items-start gap-3 transition-colors group cursor-pointer {bg_style}",
                                            
                                            // Icon indicator
                                            div { class: "mt-0.5 p-1.5 rounded-lg bg-indigo-500/10 text-indigo-400 shrink-0 border border-indigo-500/20",
                                                svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" }
                                                }
                                            }

                                            // Content
                                            div { class: "flex-1 min-w-0",
                                                div { class: "flex items-center justify-between gap-1 mb-0.5",
                                                    span { class: "text-xs font-semibold text-slate-200 truncate", "{item.title}" }
                                                    if is_unread {
                                                        span { class: "w-2 h-2 rounded-full bg-indigo-500 shrink-0" }
                                                    }
                                                }
                                                p { class: "text-xs text-slate-400 line-clamp-2 leading-snug", "{item.body}" }
                                            }

                                            // Mark read action button
                                            if is_unread {
                                                button {
                                                    r#type: "button",
                                                    class: "opacity-0 group-hover:opacity-100 text-slate-500 hover:text-indigo-400 p-1 rounded transition-all",
                                                    title: "Mark read",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        props.on_mark_read.call(notif_id.clone());
                                                    },
                                                    svg { class: "w-3.5 h-3.5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M5 13l4 4L19 7" }
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
    }
}
