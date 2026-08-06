use crate::components::{Button, ConflictResolverModal, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{
    PendingBlobUpload, SyncConflictRecord, SyncQueueSummaryRecord, get_pending_blob_uploads,
    get_sync_conflicts, get_sync_queue_breakdown, sync_database,
};

#[derive(Props, Clone, PartialEq)]
pub struct SyncLedgerDrawerProps {
    pub workspace_id: String,
    pub active_user_id: Option<String>,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn SyncLedgerDrawer(props: SyncLedgerDrawerProps) -> Element {
    let mut is_syncing = use_signal(|| false);
    let mut queue_records = use_signal(|| Vec::<SyncQueueSummaryRecord>::new());
    let mut pending_blobs = use_signal(|| Vec::<PendingBlobUpload>::new());
    let mut sync_conflicts = use_signal(|| Vec::<SyncConflictRecord>::new());
    let mut show_conflict_modal = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut active_tab = use_signal(|| "ledger".to_string());

    let ws_id1 = props.workspace_id.clone();
    let ws_id2 = props.workspace_id.clone();
    let user_id = props.active_user_id.clone().unwrap_or_else(|| "user-1".to_string());

    use_effect(move || {
        let ws = ws_id1.clone();
        spawn(async move {
            if let Ok(records) = get_sync_queue_breakdown(ws.clone()).await {
                queue_records.set(records);
            }
            if let Ok(blobs) = get_pending_blob_uploads(ws.clone()).await {
                pending_blobs.set(blobs);
            }
            if let Ok(conflicts) = get_sync_conflicts(ws).await {
                sync_conflicts.set(conflicts);
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
                    status_msg.set(Some(
                        "Database synchronization completed successfully!".to_string(),
                    ));
                    if let Ok(records) = get_sync_queue_breakdown(ws.clone()).await {
                        queue_records.set(records);
                    }
                    if let Ok(blobs) = get_pending_blob_uploads(ws.clone()).await {
                        pending_blobs.set(blobs);
                    }
                    if let Ok(conflicts) = get_sync_conflicts(ws).await {
                        sync_conflicts.set(conflicts);
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
    let conflicts = sync_conflicts.read().clone();
    let total_pending: u32 = queue.iter().map(|r| r.pending_count).sum();

    rsx! {
        div { class: "fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-xs",
            div { class: "w-full max-w-lg h-full bg-card border-l border-border p-6 shadow-2xl flex flex-col gap-5 overflow-hidden animate-in slide-in-from-right duration-200",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-sm",
                            LucideIcon { name: "activity", class: if *is_syncing.read() { "h-6 w-6 animate-spin" } else { "h-6 w-6" } }
                        }
                        div {
                            h2 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                                "Pending Sync Ledger"
                                span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20", "Loro CRDT" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Field-level offline deltas, replication queue & visual conflict drawer"
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

                // Summary KPI Bar
                div { class: "grid grid-cols-3 gap-2.5",
                    div { class: "p-3 rounded-2xl border border-border bg-background flex flex-col gap-1",
                        span { class: "text-[10px] font-medium text-muted-foreground uppercase tracking-wider", "Pending Edits" }
                        div { class: "flex items-baseline gap-1",
                            span { class: "text-xl font-bold text-foreground", "{total_pending}" }
                            span { class: "text-[10px] text-muted-foreground", "recs" }
                        }
                    }
                    div { class: "p-3 rounded-2xl border border-border bg-background flex flex-col gap-1",
                        span { class: "text-[10px] font-medium text-muted-foreground uppercase tracking-wider", "Media Blobs" }
                        div { class: "flex items-baseline gap-1",
                            span { class: "text-xl font-bold text-foreground", "{blobs.len()}" }
                            span { class: "text-[10px] text-muted-foreground", "files" }
                        }
                    }
                    div { class: "p-3 rounded-2xl border border-amber-500/30 bg-amber-500/5 flex flex-col gap-1",
                        span { class: "text-[10px] font-medium text-amber-600 uppercase tracking-wider", "CRDT Conflicts" }
                        div { class: "flex items-baseline gap-1",
                            span { class: "text-xl font-bold text-amber-600", "{conflicts.len()}" }
                            span { class: "text-[10px] text-amber-600", "diffs" }
                        }
                    }
                }

                // Tab Switcher
                div { class: "flex items-center gap-2 p-1 rounded-xl bg-secondary/50 border border-border text-xs font-semibold",
                    button {
                        class: if *active_tab.read() == "ledger" { "flex-1 py-1.5 rounded-lg bg-background text-foreground shadow-xs font-bold" } else { "flex-1 py-1.5 text-muted-foreground hover:text-foreground" },
                        onclick: move |_| active_tab.set("ledger".to_string()),
                        "Pending Write Ledger"
                    }
                    button {
                        class: if *active_tab.read() == "conflicts" { "flex-1 py-1.5 rounded-lg bg-background text-foreground shadow-xs font-bold" } else { "flex-1 py-1.5 text-muted-foreground hover:text-foreground" },
                        onclick: move |_| active_tab.set("conflicts".to_string()),
                        "CRDT Conflicts ({conflicts.len()})"
                    }
                }

                // Tab 1: Pending Write Ledger
                if *active_tab.read() == "ledger" {
                    div { class: "flex-1 overflow-y-auto space-y-4 pr-1",
                        div { class: "space-y-2",
                            h3 { class: "text-xs font-bold text-foreground uppercase tracking-wider m-0 flex items-center gap-1.5",
                                LucideIcon { name: "database", class: "h-4 w-4 text-primary" }
                                "Structured Entities Queue"
                            }
                            if queue.is_empty() {
                                div { class: "p-4 rounded-xl border border-border bg-secondary/30 text-center text-xs text-muted-foreground italic",
                                    "All entity tables are synchronized."
                                }
                            } else {
                                for rec in queue.iter() {
                                    {
                                        let tname = rec.table_name.clone();
                                        let count = rec.pending_count;
                                        rsx! {
                                            div { key: "{tname}", class: "p-3 rounded-xl border border-border bg-background flex items-center justify-between shadow-xs",
                                                div { class: "flex items-center gap-2.5",
                                                    LucideIcon { name: "layers", class: "h-4 w-4 text-muted-foreground" }
                                                    span { class: "text-xs font-bold text-foreground capitalize", "{tname}" }
                                                }
                                                span { class: "px-2.5 py-0.5 rounded-full text-xs font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20",
                                                    "{count} pending"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "space-y-2 pt-2",
                            h3 { class: "text-xs font-bold text-foreground uppercase tracking-wider m-0 flex items-center gap-1.5",
                                LucideIcon { name: "hard-drive", class: "h-4 w-4 text-primary" }
                                "Offline Media Attachments"
                            }
                            if blobs.is_empty() {
                                div { class: "p-4 rounded-xl border border-border bg-secondary/30 text-center text-xs text-muted-foreground italic",
                                    "No media uploads in queue."
                                }
                            } else {
                                for blob in blobs.iter() {
                                    {
                                        let hash = blob.hash_pointer.clone();
                                        let short_hash = if hash.len() > 12 { format!("{}...", &hash[..12]) } else { hash };
                                        rsx! {
                                            div { key: "{blob.hash_pointer}", class: "p-3 rounded-xl border border-border bg-background space-y-1.5",
                                                div { class: "flex items-center justify-between text-xs",
                                                    span { class: "font-mono font-bold text-foreground", "{short_hash}" }
                                                    span { class: "px-2 py-0.5 rounded text-[10px] font-semibold bg-primary/10 text-primary border border-primary/20",
                                                        "{blob.upload_status}"
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

                // Tab 2: Conflicts Breakdown
                if *active_tab.read() == "conflicts" {
                    div { class: "flex-1 overflow-y-auto space-y-3 pr-1",
                        if conflicts.is_empty() {
                            div { class: "p-8 text-center text-muted-foreground space-y-2",
                                LucideIcon { name: "check-circle", class: "h-8 w-8 text-emerald-500 mx-auto" }
                                h4 { class: "text-sm font-bold text-foreground m-0", "Zero CRDT Conflicts" }
                                p { class: "text-xs m-0", "Field-level local deltas are fully harmonized." }
                            }
                        } else {
                            for conflict in conflicts.iter() {
                                {
                                    let tname = conflict.table_name.clone();
                                    let rid = conflict.record_id.clone();
                                    let ctype = conflict.conflict_type.clone();
                                    let fcount = conflict.field_diffs.iter().filter(|d| d.is_conflicting).count();

                                    rsx! {
                                        div { key: "{rid}", class: "p-4 rounded-2xl border border-amber-500/30 bg-amber-500/5 space-y-2.5",
                                            div { class: "flex items-center justify-between text-xs",
                                                div { class: "flex items-center gap-2",
                                                    span { class: "font-bold text-foreground capitalize px-2 py-0.5 rounded bg-background border border-border", "{tname}" }
                                                    span { class: "font-mono text-muted-foreground font-bold", "{rid}" }
                                                }
                                                span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-amber-500/20 text-amber-700", "{ctype}" }
                                            }
                                            div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                span { "{fcount} conflicting field(s)" }
                                                button {
                                                    class: "text-xs font-bold text-amber-600 hover:text-amber-700 underline cursor-pointer",
                                                    onclick: move |_| show_conflict_modal.set(true),
                                                    "Inspect Side-by-Side Diffs →"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if *show_conflict_modal.read() {
                    ConflictResolverModal {
                        active_user_id: user_id.clone(),
                        workspace_id: props.workspace_id.clone(),
                        onresolvecomplete: move |_| {
                            show_conflict_modal.set(false);
                            let ws = props.workspace_id.clone();
                            spawn(async move {
                                if let Ok(conflicts) = get_sync_conflicts(ws).await {
                                    sync_conflicts.set(conflicts);
                                }
                            });
                        },
                        onclose: move |_| show_conflict_modal.set(false),
                    }
                }

                // Footer Actions
                div { class: "pt-4 border-t border-border/40 flex items-center gap-3",
                    if !conflicts.is_empty() {
                        Button {
                            class: "flex-1 text-xs h-10 rounded-xl border border-amber-500/40 bg-amber-500/10 text-amber-600 hover:bg-amber-500/20 font-bold flex items-center justify-center gap-2",
                            onclick: move |_| show_conflict_modal.set(true),
                            LucideIcon { name: "git-merge", class: "h-4 w-4" }
                            "Resolve Field Diffs"
                        }
                    }
                    Button {
                        class: "flex-1 text-xs h-10 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 font-bold flex items-center justify-center gap-2",
                        disabled: is_syncing,
                        onclick: trigger_manual_sync,
                        if *is_syncing.read() {
                            LucideIcon { name: "refresh-cw", class: "h-4 w-4 animate-spin" }
                            "Replicating..."
                        } else {
                            LucideIcon { name: "zap", class: "h-4 w-4" }
                            "Sync Ledger Now"
                        }
                    }
                }
            }
        }
    }
}
