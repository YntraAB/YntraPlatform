use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::export_workspace_full_data_json;

#[derive(Props, Clone, PartialEq)]
pub struct WorkspaceExportModalProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn WorkspaceExportModal(props: WorkspaceExportModalProps) -> Element {
    let mut is_exporting = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut exported_payload = use_signal(|| Option::<String>::None);

    let req_uid = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();

    let trigger_export = move |_| {
        is_exporting.set(true);
        status_msg.set(None);
        let req_uid = req_uid.clone();
        let ws_id = ws_id.clone();

        spawn(async move {
            match export_workspace_full_data_json(req_uid, ws_id).await {
                Ok(json_str) => {
                    let len_kb = (json_str.len() as f64) / 1024.0;
                    status_msg.set(Some(format!("Full workspace data archive compiled successfully ({:.1} KB)", len_kb)));
                    exported_payload.set(Some(json_str));
                }
                Err(e) => {
                    status_msg.set(Some(format!("Export failed: {}", e)));
                }
            }
            is_exporting.set(false);
        });
    };

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 sm:p-6",
            div { class: "w-full max-w-2xl rounded-3xl border border-border bg-card p-6 shadow-2xl flex flex-col gap-5 overflow-hidden",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-emerald-500/10 flex items-center justify-center text-emerald-500 shadow-sm",
                            LucideIcon { name: "download", class: "h-6 w-6" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                "1-Click Workspace Data Export"
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20", "GDPR Article 20" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Export complete workspace tables, users, time reports, notes, and audit logs"
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
                    div { class: "p-3.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-600 text-xs font-semibold flex items-center gap-2",
                        LucideIcon { name: "check-circle", class: "h-4 w-4 shrink-0" }
                        "{msg}"
                    }
                }

                // Summary Features Box
                div { class: "p-4 rounded-2xl border border-border bg-secondary/30 space-y-2.5",
                    h3 { class: "text-xs font-bold text-foreground m-0 uppercase tracking-wider", "Included Archive Collections" }
                    div { class: "grid grid-cols-2 gap-2 text-xs text-muted-foreground",
                        div { class: "flex items-center gap-2", LucideIcon { name: "users", class: "h-3.5 w-3.5 text-primary" }, "Users & Roles" }
                        div { class: "flex items-center gap-2", LucideIcon { name: "clock", class: "h-3.5 w-3.5 text-primary" }, "Time Reports & Shifts" }
                        div { class: "flex items-center gap-2", LucideIcon { name: "file-text", class: "h-3.5 w-3.5 text-primary" }, "Notes & Documents" }
                        div { class: "flex items-center gap-2", LucideIcon { name: "shield-check", class: "h-3.5 w-3.5 text-primary" }, "Cryptographic Audit Logs" }
                    }
                }

                // Preview box if compiled
                if let Some(ref payload) = *exported_payload.read() {
                    div { class: "space-y-2",
                        label { class: "text-xs font-bold text-foreground block", "Compiled Workspace JSON Archive:" }
                        textarea {
                            class: "w-full h-40 rounded-2xl border border-border bg-background p-3 text-xs font-mono text-foreground focus:outline-none resize-none",
                            readonly: true,
                            value: "{payload}"
                        }
                    }
                }

                // Action Footer
                div { class: "flex items-center justify-end gap-3 pt-3 border-t border-border/40",
                    Button {
                        class: "text-xs h-10 px-6 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer font-bold",
                        disabled: is_exporting,
                        onclick: trigger_export,
                        if *is_exporting.read() {
                            LucideIcon { name: "refresh-cw", class: "h-4 w-4 animate-spin mr-1.5" }
                            "Compiling Workspace Archive..."
                        } else {
                            LucideIcon { name: "download-cloud", class: "h-4 w-4 mr-1.5" }
                            "1-Click Compile & Export JSON"
                        }
                    }
                }
            }
        }
    }
}
