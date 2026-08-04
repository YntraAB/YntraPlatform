use dioxus::prelude::*;
use yntra_core::UserPresence;

#[derive(Props, Clone, PartialEq)]
pub struct PresenceAvatarsProps {
    pub presences: Vec<UserPresence>,
    pub current_user_id: String,
    pub max_visible: Option<usize>,
}

#[component]
pub fn PresenceAvatars(props: PresenceAvatarsProps) -> Element {
    let max = props.max_visible.unwrap_or(4);

    // Filter out current user from avatar stack if desired, or show all
    let active_peers = props.presences;
    let total = active_peers.len();
    let visible_peers: Vec<UserPresence> = active_peers.into_iter().take(max).collect();
    let overflow = total.saturating_sub(max);

    if total == 0 {
        return rsx! {};
    }

    rsx! {
        div {
            class: "flex items-center -space-x-2 overflow-hidden py-1 px-2 bg-slate-800/40 rounded-full border border-slate-700/50 backdrop-blur-sm",
            title: "{total} active team members in workspace",

            for peer in visible_peers {
                {
                    let initial = peer.user_name.chars().next().unwrap_or('U').to_uppercase().to_string();
                    let status_color = match peer.status.as_str() {
                        "away" => "bg-amber-400 border-slate-900",
                        _ => "bg-emerald-500 border-slate-900",
                    };
                    let active_info = match (&peer.active_block_id, &peer.active_view) {
                        (Some(b), Some(v)) => format!("In {} / {}", b, v),
                        (Some(b), None) => format!("In {}", b),
                        (None, Some(v)) => format!("Viewing {}", v),
                        _ => "Active".to_string(),
                    };

                    rsx! {
                        div {
                            key: "{peer.user_id}",
                            class: "relative group inline-block",

                            // Avatar circle
                            div {
                                class: "w-7 h-7 rounded-full bg-gradient-to-tr from-indigo-600 to-purple-500 flex items-center justify-center text-xs font-bold text-white shadow-md ring-2 ring-slate-900 transition-transform hover:scale-110 hover:z-20 cursor-pointer",
                                "{initial}"
                            }

                            // Status indicator dot
                            span {
                                class: "absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-2 {status_color}",
                            }

                            // Tooltip on hover
                            div {
                                class: "absolute bottom-full mb-2 left-1/2 -translate-x-1/2 hidden group-hover:flex flex-col items-center z-50 pointer-events-none",
                                div {
                                    class: "bg-slate-900 border border-slate-700 text-xs text-slate-200 py-1 px-2.5 rounded-lg shadow-xl whitespace-nowrap flex flex-col items-center gap-0.5",
                                    span { class: "font-semibold text-white", "{peer.user_name}" }
                                    span { class: "text-[10px] text-indigo-400 font-mono", "{active_info}" }
                                }
                                div { class: "w-2 h-2 bg-slate-900 border-r border-b border-slate-700 transform rotate-45 -mt-1" }
                            }
                        }
                    }
                }
            }

            if overflow > 0 {
                div {
                    class: "w-7 h-7 rounded-full bg-slate-700 text-slate-200 text-xs font-semibold flex items-center justify-center ring-2 ring-slate-900 shadow-md",
                    "+{overflow}"
                }
            }
        }
    }
}
