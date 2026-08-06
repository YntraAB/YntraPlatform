use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{
    StorageHealthReport, check_storage_health, generate_emergency_storage_backup_payload,
};

#[derive(Props, Clone, PartialEq)]
pub struct StorageGuardToastProps {
    pub active_user_id: String,
    pub workspace_id: String,
}

#[component]
pub fn StorageGuardToast(props: StorageGuardToastProps) -> Element {
    let mut health_report = use_signal(|| Option::<StorageHealthReport>::None);
    let mut is_dismissed = use_signal(|| false);
    let mut is_downloading = use_signal(|| false);
    let mut download_msg = use_signal(|| Option::<String>::None);

    let uid_eff = props.active_user_id.clone();
    let ws_eff = props.workspace_id.clone();

    use_effect(move || {
        let u = uid_eff.clone();
        let w = ws_eff.clone();
        spawn(async move {
            if let Ok(report) = check_storage_health(u, w, false, 50_000_000, 42_000_000).await {
                health_report.set(Some(report));
            }
        });
    });

    if *is_dismissed.read() {
        return rsx! {};
    }

    let report_opt = health_report.read().clone();
    let Some(report) = report_opt else {
        return rsx! {};
    };

    if !report.recommended_backup_needed && report.risk_level == "safe" {
        return rsx! {};
    }

    let is_critical = report.risk_level == "critical";
    let bg_class = if is_critical {
        "bg-red-500/10 border-red-500/30 text-red-600"
    } else {
        "bg-amber-500/10 border-amber-500/30 text-amber-600"
    };

    rsx! {
        div { class: "fixed bottom-5 right-5 z-50 w-full max-w-md p-4 rounded-2xl border shadow-xl backdrop-blur-md flex flex-col gap-3 {bg_class}",
            div { class: "flex items-start justify-between gap-3",
                div { class: "flex items-center gap-2.5",
                    LucideIcon { name: if is_critical { "alert-triangle" } else { "shield-alert" }, class: "h-5 w-5 shrink-0" }
                    div {
                        h4 { class: "text-xs font-bold m-0 uppercase tracking-wider",
                            if is_critical { "Critical Web Storage Risk" } else { "Storage Eviction Guard Warning" }
                        }
                        p { class: "text-xs opacity-90 m-0 mt-0.5",
                            if !report.is_persistent_granted {
                                "Browser persistent storage permission is denied. {report.unsynced_offline_edits_count} offline edits risk eviction under disk pressure."
                            } else {
                                "High offline edit count ({report.unsynced_offline_edits_count} pending). Download a local backup to prevent browser data loss."
                            }
                        }
                    }
                }

                button {
                    class: "p-1 rounded-lg hover:bg-black/10 transition-colors cursor-pointer",
                    onclick: move |_| is_dismissed.set(true),
                    LucideIcon { name: "x", class: "h-4 w-4" }
                }
            }

            if let Some(ref msg) = *download_msg.read() {
                div { class: "text-[11px] font-mono font-semibold px-2.5 py-1 rounded bg-black/10",
                    "{msg}"
                }
            }

            div { class: "flex items-center justify-end gap-2 pt-1 border-t border-black/10",
                Button {
                    class: "text-xs h-8 px-3 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 font-bold shadow-xs cursor-pointer",
                    disabled: is_downloading,
                    onclick: {
                        let u = props.active_user_id.clone();
                        let w = props.workspace_id.clone();
                        move |_| {
                            let u = u.clone();
                            let w = w.clone();
                            is_downloading.set(true);
                            spawn(async move {
                                if let Ok(payload) = generate_emergency_storage_backup_payload(u, w).await {
                                    download_msg.set(Some(format!("Emergency backup generated ({} records)", payload.total_pending_records)));
                                }
                                is_downloading.set(false);
                            });
                        }
                    },
                    LucideIcon { name: "download", class: "h-3.5 w-3.5 mr-1" }
                    "Emergency Backup Payload"
                }
            }
        }
    }
}
