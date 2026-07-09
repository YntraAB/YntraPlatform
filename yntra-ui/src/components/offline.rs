use crate::components::LucideIcon;
use dioxus::prelude::*;

#[component]
pub fn OfflineIndicator() -> Element {
    let mut is_online = use_signal(|| true);
    let mut is_syncing = use_signal(|| false);
    let mut pending_count = use_signal(|| 0);
    let mut has_sync_error = use_signal(|| false);

    use_effect(move || {
        let mut online_eval = document::eval(
            r#"
            window.addEventListener('online', () => dioxus.send("online"));
            window.addEventListener('offline', () => dioxus.send("offline"));
            // initial send
            dioxus.send(navigator.onLine ? "online" : "offline");
        "#,
        );

        spawn(async move {
            while let Ok(msg) = online_eval.recv::<String>().await {
                match msg.as_str() {
                    "online" => {
                        is_online.set(true);
                        is_syncing.set(true);
                        match yntra_core::sync_database().await {
                            Ok(_) => {
                                has_sync_error.set(false);
                                pending_count.set(0);
                            }
                            Err(_) => {
                                has_sync_error.set(true);
                            }
                        }
                        is_syncing.set(false);
                    }
                    "offline" => {
                        is_online.set(false);
                    }
                    _ => {}
                }
            }
        });
    });

    let on_click = move |_| {
        let online = *is_online.read();
        let syncing = *is_syncing.read();

        if online && !syncing {
            spawn(async move {
                is_syncing.set(true);
                match yntra_core::sync_database().await {
                    Ok(_) => {
                        has_sync_error.set(false);
                        pending_count.set(0);
                    }
                    Err(_) => {
                        has_sync_error.set(true);
                    }
                }
                is_syncing.set(false);
            });
        }
    };

    let online = *is_online.read();
    let syncing = *is_syncing.read();
    let error = *has_sync_error.read();
    let pending = *pending_count.read();

    if online && !syncing && !error && pending == 0 {
        return rsx! {};
    }

    let container_class = if online && pending > 0 && !syncing {
        "fixed bottom-4 left-4 z-[100] cursor-pointer hover:scale-105 transition-transform"
    } else {
        "fixed bottom-4 left-4 z-[100]"
    };

    let badge_style = if !online {
        "background-color: rgba(239, 68, 68, 0.95); color: white; border-color: rgba(248, 113, 113, 0.4);"
    } else if error {
        "background-color: rgba(245, 158, 11, 0.95); color: white; border-color: rgba(251, 191, 36, 0.4);"
    } else {
        "background-color: var(--accent); color: white; border-color: rgba(255, 255, 255, 0.1);"
    };

    rsx! {
        div { class: "{container_class}", onclick: on_click,
            div {
                class: "flex items-center gap-2 rounded-full px-4 py-2 text-xs font-bold shadow-2xl border backdrop-blur-md",
                style: "{badge_style}",
                if !online {
                    LucideIcon { name: "wifi-off", class: "h-3.5 w-3.5 animate-pulse", }
                    span {
                        "Offline Mode"
                        if pending > 0 {
                            " ({pending} pending sync changes)"
                        }
                    }
                } else if error {
                    LucideIcon {
                        name: "alert-circle",
                        class: "h-3.5 w-3.5 animate-bounce",
                    }
                    span { "Synchronization Error (click to retry)" }
                } else if syncing {
                    LucideIcon { name: "refresh-cw", class: "h-3.5 w-3.5 animate-spin", }
                    span { "Syncing local database updates... ({pending})" }
                } else {
                    LucideIcon { name: "refresh-cw", class: "h-3.5 w-3.5", }
                    span { "{pending} local changes ready to sync (click to sync)" }
                }
            }
        }
    }
}
