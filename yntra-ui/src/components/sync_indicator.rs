use crate::components::LucideIcon;
use dioxus::prelude::*;
use yntra_core::get_sync_queue_status;

#[derive(Props, Clone, PartialEq)]
pub struct SyncIndicatorProps {
    pub active_user_id: Signal<String>,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
    pub on_open_drawer: EventHandler<()>,
}

#[component]
pub fn SyncIndicator(props: SyncIndicatorProps) -> Element {
    let mut pending_count = use_signal(|| 0u32);
    let mut quarantined_count = use_signal(|| 0u32);
    let mut sync_state = use_signal(|| "synced".to_string());

    let uid = props.active_user_id.read().clone();
    let ws_id = props.workspace_id.clone();
    let trigger = props.db_trigger;

    use_effect(move || {
        let _t = trigger.read();
        let u = uid.clone();
        let w = ws_id.clone();
        spawn(async move {
            if let Ok(status) = get_sync_queue_status(u, w).await {
                pending_count.set(status.pending_changes_count);
                quarantined_count.set(status.quarantined_conflicts_count);
                sync_state.set(status.state);
            }
        });
    });

    let on_click = move |_| {
        props.on_open_drawer.call(());
    };

    let (badge_class, icon_name, text_label, sub_label) = match sync_state.read().as_str() {
        "has_conflicts" => (
            "bg-rose-500/10 text-rose-500 border-rose-500/30 hover:bg-rose-500/20 shadow-rose-500/10",
            "alert-triangle",
            format!("{} Conflicts", quarantined_count.read()),
            "Action Required",
        ),
        "pending_upload" => (
            "bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/30 hover:bg-amber-500/20 shadow-amber-500/10",
            "cloud-upload",
            format!("Saved Locally ({} pending)", pending_count.read()),
            "Queue Active",
        ),
        _ => (
            "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/30 hover:bg-emerald-500/20 shadow-emerald-500/10",
            "check-circle-2",
            "Synced (0ms)".to_string(),
            "Zero-Delay Engine",
        ),
    };

    rsx! {
        button {
            class: "group relative flex items-center space-x-2 px-3 py-1.5 rounded-full border text-xs font-semibold transition-all cursor-pointer shadow-xs {badge_class}",
            onclick: on_click,
            title: "Click to open Local-First Sync Monitor & Queue Drawer",
            div { class: "relative flex items-center justify-center",
                if sync_state.read().as_str() == "synced" {
                    span { class: "absolute inline-flex h-2 w-2 rounded-full bg-emerald-400 opacity-75 animate-ping" }
                }
                LucideIcon { name: "{icon_name}", class: "w-3.5 h-3.5 shrink-0" }
            }
            span { class: "tracking-tight", "{text_label}" }
            span { class: "text-[10px] font-bold uppercase opacity-60 px-1.5 py-0.2 rounded bg-black/10 hidden sm:inline-block", "{sub_label}" }
        }
    }
}

