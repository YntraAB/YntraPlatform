use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{get_pending_blob_uploads, get_sync_queue_breakdown, sync_database, PendingBlobUpload, SyncQueueSummaryRecord};

#[derive(Props, Clone, PartialEq)]
pub struct SyncMonitorDrawerProps {
    pub workspace_id: String,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn SyncMonitorDrawer(props: SyncMonitorDrawerProps) -> Element {
    let mut is_syncing = use_signal(|| false);
    let mut queue_records = use_signal(|| Vec::<SyncQueueSummaryRecord>::new());
    let mut pending_blobs = use_signal(|| Vec::<PendingBlobUpload>::new());
    let mut status_msg = use_signal(|| Option::<String>::None);

    let ws_id1 = props.workspace_id.clone();
    let ws_id2 = props.workspace_id.clone();

    use_effect(move || {
        let ws = ws_id1.clone();
        spawn(async move {
            if let Ok(records) = get_sync_queue_breakdown(ws.clone()).await {
                queue_records.set(records);
            }
            if let Ok(blobs) = get_pending_blob_uploads(ws).await {
                pending_blobs.set(blobs);
            }
        });
    });

    let trigger_manual_sync = move |_| {
        is_syncing.set(true);
        status_msg.set(None);
        let ws = ws_id2.clone();

        spawn(async move {
            match sync_database().await {
                Ok(_) => {
                    status_msg.set(Some("Database synchronization completed successfully!".to_string()));
                    if let Ok(records) = get_sync_queue_breakdown(ws.clone()).await {
                        queue_records.set(records);
                    }
                    if let Ok(blobs) = get_pending_blob_uploads(ws).await {
                        pending_blobs.set(blobs);
                    }
                }
                Err(e) => {
                    status_msg.set(Some(format!("Sync failed: {}", e)));
                }
            }
            is_syncing.set(false);
        });
    };

    let queue = queue_records.read().clone();
    let blobs = pending_blobs.read().clone();
    let total_pending: u32 = queue.iter().map(|r| r.pending_count).sum();

    rsx! {
        div { class: "fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-xs",
            div { class: "w-full max-w-md h-full bg-card border-l border-border p-6 shadow-2xl flex flex-col gap-5 overflow-hidden animate-in slide-in-from-right duration-200",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-10 w-10 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-sm",
                            LucideIcon { name: "refresh-cw", class: if *is_syncing.read() { "h-5 w-5 animate-spin" } else { "h-5 w-5" } }
                        }
                        div {
                            h2 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                                "Local-First Sync Monitor"
                                span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20", "Sub-ms Engine" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Real-time libSQL replication & offline media upload queue"
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
                        LucideIcon { name: "info", class: "h-4 w-4 shrink-0" }
                        "{msg}"
                    }
                }

                // Summary Stats Card
                div { class: "grid grid-cols-2 gap-3",
                    div { class: "p-3.5 rounded-2xl border border-border bg-background flex flex-col gap-1",
                        span { class: "text-[11px] font-medium text-muted-foreground uppercase tracking-wider", "Pending Structured Records" }
                        div { class: "flex items-baseline gap-1.5",
                            span { class: "text-2xl font-bold text-foreground", "{total_pending}" }
                            span { class: "text-xs text-muted-foreground", "items" }
                        }
                    }
                    div { class: "p-3.5 rounded-2xl border border-border bg-background flex flex-col gap-1",
                        span { class: "text-[11px] font-medium text-muted-foreground uppercase tracking-wider", "Queued Media Blobs" }
                        div { class: "flex items-baseline gap-1.5",
                            span { class: "text-2xl font-bold text-foreground", "{blobs.len()}" }
                            span { class: "text-xs text-muted-foreground", "files" }
                        }
                    }
                }

                // Scrollable List
                div { class: "flex-1 overflow-y-auto space-y-5 pr-1",
                    // Section 1: Structured DB Tables Queue
                    div { class: "space-y-2.5",
                        h3 { class: "text-xs font-bold text-foreground uppercase tracking-wider m-0 flex items-center gap-1.5",
                            LucideIcon { name: "database", class: "h-4 w-4 text-primary" }
                            "Structured Database Queue"
                        }

                        if queue.is_empty() {
                            div { class: "p-4 rounded-xl border border-border bg-secondary/30 text-center text-xs text-muted-foreground italic",
                                "All local database tables are fully synchronized with cloud!"
                            }
                        } else {
                            div { class: "space-y-2",
                                for rec in queue.iter() {
                                    {
                                        let table_name = rec.table_name.clone();
                                        let pending = rec.pending_count;
                                        rsx! {
                                            div { key: "{table_name}", class: "p-3 rounded-xl border border-border bg-background flex items-center justify-between shadow-xs",
                                                div { class: "flex items-center gap-2.5",
                                                    LucideIcon { name: "layers", class: "h-4 w-4 text-muted-foreground" }
                                                    span { class: "text-xs font-bold text-foreground capitalize", "{table_name}" }
                                                }
                                                span { class: "px-2.5 py-0.5 rounded-full text-xs font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20",
                                                    "{pending} pending"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Section 2: Media Blobs Queue
                    div { class: "space-y-2.5 pt-2",
                        h3 { class: "text-xs font-bold text-foreground uppercase tracking-wider m-0 flex items-center gap-1.5",
                            LucideIcon { name: "hard-drive", class: "h-4 w-4 text-primary" }
                            "Offline Media Upload Queue"
                        }

                        if blobs.is_empty() {
                            div { class: "p-4 rounded-xl border border-border bg-secondary/30 text-center text-xs text-muted-foreground italic",
                                "No media attachments pending upload."
                            }
                        } else {
                            div { class: "space-y-2",
                                for blob in blobs.iter() {
                                    {
                                        let hash = blob.hash_pointer.clone();
                                        let short_hash = if hash.len() > 12 { format!("{}...", &hash[..12]) } else { hash };
                                        let orig_mb = (blob.original_size_bytes as f64) / 1024.0 / 1024.0;
                                        let comp_mb = (blob.compressed_size_bytes as f64) / 1024.0 / 1024.0;

                                        rsx! {
                                            div { key: "{blob.hash_pointer}", class: "p-3 rounded-xl border border-border bg-background space-y-2",
                                                div { class: "flex items-center justify-between text-xs",
                                                    span { class: "font-mono font-bold text-foreground", "{short_hash}" }
                                                    span { class: "px-2 py-0.5 rounded text-[10px] font-semibold bg-primary/10 text-primary border border-primary/20",
                                                        "{blob.upload_status}"
                                                    }
                                                }
                                                div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                                    span { "Type: {blob.media_type}" }
                                                    span { "Compressed: {comp_mb:.2} MB (saved {orig_mb - comp_mb:.2} MB)" }
                                                }
                                                div { class: "w-full h-1.5 rounded-full bg-secondary overflow-hidden",
                                                    div { class: "h-full bg-primary animate-pulse w-3/4 rounded-full" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer Action
                div { class: "pt-4 border-t border-border/40",
                    Button {
                        class: "w-full text-xs h-10 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer flex items-center justify-center gap-2",
                        disabled: is_syncing,
                        onclick: trigger_manual_sync,
                        if *is_syncing.read() {
                            LucideIcon { name: "refresh-cw", class: "h-4 w-4 animate-spin" }
                            "Replicating Database Edits..."
                        } else {
                            LucideIcon { name: "zap", class: "h-4 w-4" }
                            "Trigger Sub-ms Sync Now"
                        }
                    }
                }
            }
        }
    }
}
