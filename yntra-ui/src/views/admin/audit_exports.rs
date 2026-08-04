use crate::components;
use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct AuditExportsProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn AuditExportsView(props: AuditExportsProps) -> Element {
    let mut filter_action = use_signal(String::new);
    let mut export_format = use_signal(|| "csv".to_string());

    let mut is_exporting = use_signal(|| false);
    let mut export_output = use_signal(|| Option::<String>::None);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let active_user_id = props.active_user.id.clone();
    let mut db_trigger = props.db_trigger;

    // Asynchronous resource for audit log list
    let audit_logs_res = use_resource(move || {
        let uid = active_user_id.clone();
        let _trig = *db_trigger.read();
        async move { yntra_core::get_audit_logs(uid).await }
    });

    let active_uid_verify = props.active_user.id.clone();
    // Asynchronous resource for chain verification check
    let chain_verify_res = use_resource(move || {
        let uid = active_uid_verify.clone();
        let _trig = *db_trigger.read();
        async move { yntra_core::verify_audit_log_chain(uid).await }
    });

    let active_uid_export = props.active_user.id.clone();

    let handle_export = move |_| {
        is_exporting.set(true);
        error_msg.set(None);
        status_msg.set(None);
        export_output.set(None);

        let uid = active_uid_export.clone();
        let fmt = export_format.read().clone();
        let act_filter = if filter_action.read().trim().is_empty() {
            None
        } else {
            Some(filter_action.read().trim().to_string())
        };

        spawn(async move {
            if fmt == "json" {
                match yntra_core::export_audit_logs_json(uid, None, None, act_filter).await {
                    Ok(json_str) => {
                        is_exporting.set(false);
                        export_output.set(Some(json_str));
                        status_msg.set(Some(
                            "Audit logs successfully exported as formatted JSON!".to_string(),
                        ));
                    }
                    Err(e) => {
                        is_exporting.set(false);
                        error_msg.set(Some(format!("JSON export failed: {}", e)));
                    }
                }
            } else {
                match yntra_core::export_audit_logs_csv(uid, None, None, act_filter).await {
                    Ok(csv_str) => {
                        is_exporting.set(false);
                        export_output.set(Some(csv_str));
                        status_msg
                            .set(Some("Audit logs successfully exported as CSV!".to_string()));
                    }
                    Err(e) => {
                        is_exporting.set(false);
                        error_msg.set(Some(format!("CSV export failed: {}", e)));
                    }
                }
            }
        });
    };

    rsx! {
        div { class: "p-6 space-y-8 animate-in fade-in duration-300",
            // Section Header
            div { class: "flex items-center justify-between border-b border-border/40 pb-4",
                div { class: "space-y-1",
                    h2 { class: "text-2xl font-extrabold text-foreground tracking-tight flex items-center gap-2",
                        components::LucideIcon { name: "file-text", class: "h-6 w-6 text-primary" }
                        "Audit Log & Security Compliance Exports"
                    }
                    p { class: "text-xs text-muted-foreground",
                        "Inspect BLAKE3 tamper-evident audit logs and export cryptographically verified reports."
                    }
                }

                // Chain Verification Indicator
                match &*chain_verify_res.read() {
                    Some(Ok(true)) => rsx! {
                        div { class: "inline-flex items-center gap-2 px-3 py-1.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-500 text-xs font-bold shadow-xs",
                            components::LucideIcon { name: "shield-check", class: "h-4 w-4" }
                            "Hash Chain Verified (OK)"
                        }
                    },
                    Some(Ok(false)) => rsx! {
                        div { class: "inline-flex items-center gap-2 px-3 py-1.5 rounded-xl bg-red-500/10 border border-red-500/30 text-red-500 text-xs font-bold shadow-xs",
                            components::LucideIcon { name: "shield-alert", class: "h-4 w-4" }
                            "Chain Tamper Warning"
                        }
                    },
                    _ => rsx! {
                        div { class: "inline-flex items-center gap-2 px-3 py-1.5 rounded-xl bg-muted border border-border/40 text-muted-foreground text-xs font-medium animate-pulse",
                            "Verifying Hash Chain..."
                        }
                    }
                }
            }

            // Export Controls Card
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-6",
                h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                    components::LucideIcon { name: "download", class: "h-4 w-4 text-primary" }
                    "Export Audit Logs"
                }

                if let Some(err) = error_msg.read().as_ref() {
                    div { class: "p-3.5 rounded-xl bg-red-500/10 border border-red-500/30 text-red-500 text-xs font-medium flex items-center gap-2.5",
                        components::LucideIcon { name: "alert-circle", class: "h-4 w-4 shrink-0" }
                        span { "{err}" }
                    }
                }

                if let Some(msg) = status_msg.read().as_ref() {
                    div { class: "p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-500 text-xs font-medium flex items-center gap-2.5",
                        components::LucideIcon { name: "check-circle", class: "h-4 w-4 shrink-0" }
                        span { "{msg}" }
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4 items-end",
                    div { class: "space-y-1.5",
                        label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Action Filter (Optional)" }
                        input {
                            class: "w-full px-4 py-2 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                            placeholder: "e.g. read_medications, update_role",
                            value: "{filter_action.read()}",
                            oninput: move |e| filter_action.set(e.value())
                        }
                    }

                    div { class: "space-y-1.5",
                        label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Export Format" }
                        select {
                            class: "w-full px-3 py-2 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                            value: "{export_format.read()}",
                            onchange: move |e| export_format.set(e.value()),
                            option { value: "csv", "CSV (Comma Separated Values)" }
                            option { value: "json", "JSON (Formatted Payload)" }
                        }
                    }

                    div { class: "flex items-center gap-3",
                        button {
                            class: "w-full py-2.5 rounded-xl bg-primary text-primary-foreground font-bold text-sm hover:opacity-90 transition-all flex items-center justify-center gap-2 shadow-sm disabled:opacity-50",
                            disabled: *is_exporting.read(),
                            onclick: handle_export,
                            if *is_exporting.read() {
                                components::LucideIcon { name: "loader-2", class: "h-4 w-4 animate-spin" }
                                span { "Exporting..." }
                            } else {
                                components::LucideIcon { name: "file-spread-sheet", class: "h-4 w-4" }
                                span { "Generate Export" }
                            }
                        }
                    }
                }

                // Export Output Text Area if generated
                if let Some(out) = export_output.read().as_ref() {
                    div { class: "space-y-2 pt-2 animate-in fade-in duration-200",
                        div { class: "flex items-center justify-between",
                            label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Exported Data Preview" }
                            span { class: "text-xs text-muted-foreground font-mono", "{out.len()} bytes" }
                        }
                        textarea {
                            class: "w-full h-44 p-3 rounded-xl border border-border/40 bg-muted/40 font-mono text-xs text-foreground focus:outline-none resize-none shadow-inner",
                            readonly: true,
                            value: "{out}"
                        }
                    }
                }
            }

            // Audit Logs Table Card
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-4",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "activity", class: "h-4 w-4 text-primary" }
                        "Audit Stream"
                    }
                    button {
                        class: "p-2 rounded-xl border border-input bg-background text-xs font-medium hover:bg-accent transition-colors flex items-center gap-1.5",
                        onclick: move |_| {
                            let val = *db_trigger.read();
                            db_trigger.set(val + 1);
                        },
                        components::LucideIcon { name: "refresh-cw", class: "h-3.5 w-3.5" }
                        "Refresh Stream"
                    }
                }

                match &*audit_logs_res.read() {
                    Some(Ok(logs)) if !logs.is_empty() => rsx! {
                        div { class: "overflow-x-auto rounded-xl border border-border/30 bg-background",
                            table { class: "w-full text-left border-collapse",
                                thead {
                                    tr { class: "border-b border-border/40 bg-muted/40 text-xs font-semibold text-muted-foreground uppercase tracking-wider",
                                        th { class: "p-3.5", "Seq #" }
                                        th { class: "p-3.5", "Action" }
                                        th { class: "p-3.5", "Actor ID" }
                                        th { class: "p-3.5", "Timestamp" }
                                        th { class: "p-3.5", "Current Hash" }
                                        th { class: "p-3.5 text-right", "Signature" }
                                    }
                                }
                                tbody { class: "divide-y divide-border/20 text-xs text-foreground font-mono",
                                    for entry in logs.iter() {
                                        tr { key: "{entry.id}", class: "hover:bg-muted/20 transition-colors",
                                                td { class: "p-3.5 font-bold text-primary", "#{entry.seq}" }
                                                td { class: "p-3.5 font-sans font-semibold text-foreground", "{entry.action_type}" }
                                                td { class: "p-3.5 text-muted-foreground", "{entry.actor_id}" }
                                                td { class: "p-3.5 text-muted-foreground font-sans", "{entry.timestamp}" }
                                                td { class: "p-3.5 text-muted-foreground text-xs font-mono truncate max-w-xs", "{entry.curr_hash}" }
                                                td { class: "p-3.5 text-right font-sans",
                                                    if entry.signature.is_some() {
                                                        span { class: "px-2 py-0.5 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-500 border border-emerald-500/20",
                                                            "Ed25519"
                                                        }
                                                    } else {
                                                        span { class: "text-muted-foreground/60 text-xs", "Unsigned" }
                                                    }
                                                }
                                            }
                                    }
                                }
                            }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        div { class: "py-12 text-center border border-dashed border-border/60 rounded-xl bg-muted/20 text-muted-foreground space-y-2",
                            components::LucideIcon { name: "shield-off", class: "h-8 w-8 mx-auto text-muted-foreground/60" }
                            p { class: "text-sm font-medium m-0", "No audit log entries recorded yet" }
                        }
                    },
                    Some(Err(err)) => rsx! {
                        div { class: "p-4 rounded-xl bg-red-500/10 text-red-500 text-xs font-medium",
                            "Failed to load audit logs: {err}"
                        }
                    },
                    None => rsx! {
                        div { class: "py-8 text-center text-muted-foreground text-xs animate-pulse",
                            "Loading audit log entries..."
                        }
                    }
                }
            }
        }
    }
}
