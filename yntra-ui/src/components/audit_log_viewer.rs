use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{
    AuditLogEntry, export_audit_logs_csv, export_audit_logs_json, get_audit_logs,
    verify_audit_log_chain,
};

#[derive(Props, Clone, PartialEq)]
pub struct AuditLogViewerProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn AuditLogViewer(props: AuditLogViewerProps) -> Element {
    let mut logs_state = use_signal(|| Vec::<AuditLogEntry>::new());
    let mut chain_verified = use_signal(|| Option::<bool>::None);
    let mut search_filter = use_signal(|| String::new());
    let mut action_filter = use_signal(|| "All".to_string());
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| true);

    let req_uid = props.active_user_id.clone();
    let load_audit_data = move || {
        let req_uid = req_uid.clone();
        spawn(async move {
            is_loading.set(true);
            if let Ok(logs) = get_audit_logs(req_uid.clone()).await {
                logs_state.set(logs);
            }
            if let Ok(is_valid) = verify_audit_log_chain(req_uid.clone()).await {
                chain_verified.set(Some(is_valid));
            }
            is_loading.set(false);
        });
    };

    use_effect(move || {
        load_audit_data();
    });

    let uid_csv = props.active_user_id.clone();
    let handle_export_csv = move |_| {
        let req_uid = uid_csv.clone();
        spawn(async move {
            if let Ok(csv) = export_audit_logs_csv(req_uid, None, None, None).await {
                status_msg.set(Some(format!(
                    "Audit logs exported to CSV ({} bytes)",
                    csv.len()
                )));
            }
        });
    };

    let uid_json = props.active_user_id.clone();
    let handle_export_json = move |_| {
        let req_uid = uid_json.clone();
        spawn(async move {
            if let Ok(json) = export_audit_logs_json(req_uid, None, None, None).await {
                status_msg.set(Some(format!(
                    "Audit logs exported to JSON ({} bytes)",
                    json.len()
                )));
            }
        });
    };

    let logs = logs_state.read().clone();
    let search = search_filter.read().to_lowercase();
    let act_filt = action_filter.read().clone();

    let filtered_logs: Vec<AuditLogEntry> = logs
        .into_iter()
        .filter(|log| {
            let match_search = search.is_empty()
                || log.actor_id.to_lowercase().contains(&search)
                || log.action_type.to_lowercase().contains(&search)
                || log.curr_hash.to_lowercase().contains(&search);
            let match_act = act_filt == "All" || log.action_type == act_filt;
            match_search && match_act
        })
        .collect();

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 sm:p-6",
            div { class: "w-full max-w-5xl h-[85vh] rounded-3xl border border-border bg-card p-6 shadow-2xl flex flex-col gap-5 overflow-hidden",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-sm",
                            LucideIcon { name: "shield-check", class: "h-6 w-6" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                "Tamper-Proof Audit Log Viewer"
                                if let Some(verified) = *chain_verified.read() {
                                    if verified {
                                        span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 flex items-center gap-1",
                                            LucideIcon { name: "lock", class: "h-3 w-3" }
                                            "Ed25519 Chain Verified"
                                        }
                                        span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-primary/10 text-primary border border-primary/20 flex items-center gap-1",
                                            LucideIcon { name: "clock", class: "h-3 w-3" }
                                            "Monotonic TSA Vector Clock"
                                        }
                                    } else {
                                        span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-destructive/10 text-destructive border border-destructive/20",
                                            "Chain Tampered"
                                        }
                                    }
                                }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Cryptographically signed BLAKE3 hash chain of all administrative and data mutation actions"
                            }
                        }
                    }

                    button {
                        class: "p-2 rounded-xl border border-border bg-secondary text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                        onclick: move |_| props.onclose.call(()),
                        LucideIcon { name: "x", class: "h-5 w-5" }
                    }
                }

                if let Some(ref msg) = *status_msg.read() {
                    div { class: "p-3 rounded-xl border border-primary/30 bg-primary/10 text-primary text-xs font-semibold flex items-center gap-2",
                        LucideIcon { name: "check-circle", class: "h-4 w-4 shrink-0" }
                        "{msg}"
                    }
                }

                // Controls & Filter Bar
                div { class: "flex flex-wrap items-center justify-between gap-3 pb-1 border-b border-border/40",
                    div { class: "flex items-center gap-2 flex-1 max-w-sm",
                        div { class: "relative w-full",
                            input {
                                class: "w-full h-9 pl-9 pr-3 rounded-xl border border-border bg-background text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                placeholder: "Search actor, action type, hash...",
                                value: "{search_filter}",
                                oninput: move |e| search_filter.set(e.value())
                            }
                            div { class: "absolute left-3 top-2.5 text-muted-foreground",
                                LucideIcon { name: "search", class: "h-4 w-4" }
                            }
                        }
                    }

                    div { class: "flex items-center gap-2",
                        Button {
                            class: "text-xs h-9 px-3 rounded-xl border border-border bg-secondary text-secondary-foreground hover:bg-secondary/80 cursor-pointer",
                            onclick: handle_export_csv,
                            LucideIcon { name: "download", class: "h-3.5 w-3.5 mr-1" }
                            "Export CSV"
                        }
                        Button {
                            class: "text-xs h-9 px-3 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer",
                            onclick: handle_export_json,
                            LucideIcon { name: "file-code", class: "h-3.5 w-3.5 mr-1" }
                            "Export JSON"
                        }
                    }
                }

                // Audit Log Table
                div { class: "flex-1 overflow-auto rounded-2xl border border-border bg-background",
                    if *is_loading.read() {
                        div { class: "p-12 text-center text-muted-foreground italic", "Loading cryptographic audit log chain..." }
                    } else if filtered_logs.is_empty() {
                        div { class: "p-12 text-center text-muted-foreground italic", "No audit log records match your filter criteria." }
                    } else {
                        table { class: "w-full text-left text-xs border-collapse",
                            thead { class: "bg-secondary/60 text-muted-foreground font-semibold border-b border-border sticky top-0 backdrop-blur-md",
                                tr {
                                    th { class: "p-3 font-mono", "Seq #" }
                                    th { class: "p-3", "Actor ID" }
                                    th { class: "p-3", "Action Type" }
                                    th { class: "p-3", "Timestamp" }
                                    th { class: "p-3 font-mono", "BLAKE3 Hash" }
                                    th { class: "p-3 font-mono", "Ed25519 Sig" }
                                }
                            }
                            tbody { class: "divide-y divide-border/60 text-foreground",
                                for entry in filtered_logs.iter() {
                                    {
                                        let short_hash = if entry.curr_hash.len() > 14 { format!("{}...", &entry.curr_hash[..14]) } else { entry.curr_hash.clone() };
                                        let short_sig = entry.signature.as_ref().map(|s| if s.len() > 12 { format!("{}...", &s[..12]) } else { s.clone() }).unwrap_or_else(|| "Unsigned".to_string());

                                        rsx! {
                                            tr { key: "{entry.id}", class: "hover:bg-muted/30 transition-colors font-mono text-[11px]",
                                                td { class: "p-3 font-bold text-primary", "#{entry.seq}" }
                                                td { class: "p-3 font-sans font-semibold text-foreground", "{entry.actor_id}" }
                                                td { class: "p-3 font-sans",
                                                    span { class: "px-2 py-0.5 rounded-md font-bold bg-secondary text-secondary-foreground border border-border",
                                                        "{entry.action_type}"
                                                    }
                                                }
                                                td { class: "p-3 font-sans text-muted-foreground", "{entry.timestamp}" }
                                                td { class: "p-3 text-muted-foreground", "{short_hash}" }
                                                td { class: "p-3 text-emerald-600 font-semibold", "{short_sig}" }
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
