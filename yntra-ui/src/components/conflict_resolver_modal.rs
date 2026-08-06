use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{SyncConflictRecord, get_sync_conflicts, resolve_sync_conflict};

#[derive(Props, Clone, PartialEq)]
pub struct ConflictResolverModalProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub onresolvecomplete: EventHandler<()>,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn ConflictResolverModal(props: ConflictResolverModalProps) -> Element {
    let mut conflicts_state = use_signal(|| Vec::<SyncConflictRecord>::new());
    let mut selected_index = use_signal(|| 0usize);
    let mut is_resolving = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut custom_merge_mode = use_signal(|| false);

    let uid_eff = props.active_user_id.clone();
    let ws_eff = props.workspace_id.clone();

    use_effect(move || {
        let u = uid_eff.clone();
        let w = ws_eff.clone();
        spawn(async move {
            if let Ok(list) = get_sync_conflicts(w).await {
                conflicts_state.set(list);
            }
        });
    });

    let conflicts = conflicts_state.read().clone();
    let idx = *selected_index.read();

    let (local_pretty, remote_pretty, severity, conflict_type) = if let Some(conflict) = conflicts.get(idx) {
        let l = serde_json::from_str::<serde_json::Value>(&conflict.local_version_json)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|_| conflict.local_version_json.clone());
        let r = serde_json::from_str::<serde_json::Value>(&conflict.remote_version_json)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|_| conflict.remote_version_json.clone());
        (l, r, conflict.severity.clone(), conflict.conflict_type.clone())
    } else {
        (String::new(), String::new(), "medium".to_string(), "unknown".to_string())
    };

    let sev_badge_class = match severity.to_lowercase().as_str() {
        "critical" | "high" => "bg-red-500/10 text-red-600 border-red-500/20",
        "medium" => "bg-amber-500/10 text-amber-600 border-amber-500/20",
        _ => "bg-blue-500/10 text-blue-600 border-blue-500/20",
    };

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 sm:p-6",
            div { class: "w-full max-w-4xl rounded-3xl border border-border bg-card p-6 shadow-2xl flex flex-col gap-5 overflow-hidden",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-amber-500/10 flex items-center justify-center text-amber-500 shadow-sm",
                            LucideIcon { name: "git-merge", class: "h-6 w-6" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                "Visual Offline Conflict Quarantine Center"
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-amber-500/10 text-amber-600 border border-amber-500/20", "Offline Conflict" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Review quarantined offline edits and perform side-by-side verification to avoid data overwrites."
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

                if conflicts.is_empty() {
                    div { class: "p-12 text-center text-muted-foreground space-y-2",
                        LucideIcon { name: "check-circle", class: "h-10 w-10 text-emerald-500 mx-auto" }
                        h3 { class: "text-base font-bold text-foreground m-0", "Zero Conflicts Quarantined" }
                        p { class: "text-xs m-0", "All offline operational records and database replicas are synchronized." }
                    }
                } else if let Some(conflict) = conflicts.get(idx) {
                    div { class: "space-y-4",
                        // Meta info bar
                        div { class: "p-3 rounded-xl border border-border bg-secondary/30 flex items-center justify-between text-xs",
                            div { class: "flex items-center gap-2 flex-wrap",
                                span { class: "font-semibold text-muted-foreground", "Entity Table:" }
                                span { class: "font-bold text-foreground capitalize px-2 py-0.5 rounded bg-background border border-border", "{conflict.table_name}" }
                                span { class: "font-semibold text-muted-foreground ml-2", "ID:" }
                                span { class: "font-mono font-bold text-foreground", "{conflict.record_id}" }
                                span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase border ml-2 {sev_badge_class}", "{severity}" }
                                span { class: "px-2 py-0.5 rounded text-[10px] font-mono bg-secondary text-secondary-foreground border border-border ml-1", "{conflict_type}" }
                            }
                            span { class: "text-muted-foreground font-semibold", "Conflict #{idx + 1} of {conflicts.len()}" }
                        }

                        // Mode Selector / Info Bar
                        div { class: "flex items-center justify-between border-b border-border/60 pb-2",
                            div { class: "flex items-center gap-2",
                                button {
                                    class: if !*custom_merge_mode.read() { "px-3 py-1 rounded-lg text-xs font-bold bg-primary text-primary-foreground shadow-xs" } else { "px-3 py-1 rounded-lg text-xs font-semibold bg-secondary text-secondary-foreground hover:bg-secondary/80" },
                                    onclick: move |_| custom_merge_mode.set(false),
                                    "Field-by-Field CRDT Diffs"
                                }
                                button {
                                    class: if *custom_merge_mode.read() { "px-3 py-1 rounded-lg text-xs font-bold bg-primary text-primary-foreground shadow-xs" } else { "px-3 py-1 rounded-lg text-xs font-semibold bg-secondary text-secondary-foreground hover:bg-secondary/80" },
                                    onclick: move |_| custom_merge_mode.set(true),
                                    "Raw JSON Payloads"
                                }
                            }
                            span { class: "text-[11px] font-semibold text-muted-foreground",
                                "{conflict.field_diffs.iter().filter(|d| d.is_conflicting).count()} conflicting field(s)"
                            }
                        }

                        if !*custom_merge_mode.read() && !conflict.field_diffs.is_empty() {
                            // Field-by-Field Breakdown
                            div { class: "max-h-72 overflow-y-auto space-y-3 pr-1",
                                for diff in conflict.field_diffs.iter() {
                                    {
                                        let fname = diff.field_name.clone();
                                        let lval = diff.local_value.clone();
                                        let rval = diff.remote_value.clone();
                                        let is_conf = diff.is_conflicting;

                                        rsx! {
                                            div { key: "{fname}", class: "p-3 rounded-2xl border border-border bg-background space-y-2",
                                                div { class: "flex items-center justify-between text-xs",
                                                    div { class: "flex items-center gap-2",
                                                        span { class: "font-mono font-bold text-foreground capitalize", "{fname}" }
                                                        if is_conf {
                                                            span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20", "Conflicting Edit" }
                                                        } else {
                                                            span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-500/10 text-emerald-600 border border-emerald-500/20", "Clean Field" }
                                                        }
                                                    }
                                                }
                                                div { class: "grid grid-cols-1 md:grid-cols-2 gap-3 text-xs font-mono",
                                                    div { class: "p-2.5 rounded-xl border border-emerald-500/20 bg-emerald-500/5 text-foreground overflow-x-auto",
                                                        span { class: "text-[10px] font-bold uppercase text-emerald-600 block mb-1", "Local Field Value" }
                                                        "{lval}"
                                                    }
                                                    div { class: "p-2.5 rounded-xl border border-primary/20 bg-primary/5 text-foreground overflow-x-auto",
                                                        span { class: "text-[10px] font-bold uppercase text-primary block mb-1", "Cloud Server Value" }
                                                        "{rval}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Side-by-side raw diff viewer
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                // Local Version
                                div { class: "rounded-2xl border border-emerald-500/30 bg-emerald-500/5 p-4 space-y-2",
                                    div { class: "flex items-center justify-between border-b border-emerald-500/20 pb-2",
                                        span { class: "text-xs font-bold text-emerald-600 flex items-center gap-1.5",
                                            LucideIcon { name: "laptop", class: "h-4 w-4" }
                                            "Local Device Offline State"
                                        }
                                        span { class: "text-[10px] font-bold uppercase px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-700", "Local Edit" }
                                    }
                                    pre { class: "w-full h-48 rounded-xl border border-emerald-500/20 bg-background p-3 text-xs font-mono text-foreground overflow-auto",
                                        "{local_pretty}"
                                    }
                                }

                                // Cloud Server Version
                                div { class: "rounded-2xl border border-primary/30 bg-primary/5 p-4 space-y-2",
                                    div { class: "flex items-center justify-between border-b border-primary/20 pb-2",
                                        span { class: "text-xs font-bold text-primary flex items-center gap-1.5",
                                            LucideIcon { name: "cloud", class: "h-4 w-4" }
                                            "Cloud Replication Payload"
                                        }
                                        span { class: "text-[10px] font-bold uppercase px-2 py-0.5 rounded bg-primary/20 text-primary", "Server Payload" }
                                    }
                                    pre { class: "w-full h-48 rounded-xl border border-primary/20 bg-background p-3 text-xs font-mono text-foreground overflow-auto",
                                        "{remote_pretty}"
                                    }
                                }
                            }
                        }


                        // Action buttons
                        div { class: "flex items-center justify-between pt-3 border-t border-border/40 flex-wrap gap-3",
                            div { class: "flex items-center gap-2",
                                if conflicts.len() > 1 {
                                    Button {
                                        class: "text-xs h-9 px-3 rounded-xl border border-border bg-secondary text-secondary-foreground hover:bg-secondary/80",
                                        disabled: idx == 0,
                                        onclick: move |_| selected_index.set(idx.saturating_sub(1)),
                                        "Previous"
                                    }
                                    Button {
                                        class: "text-xs h-9 px-3 rounded-xl border border-border bg-secondary text-secondary-foreground hover:bg-secondary/80",
                                        disabled: idx + 1 >= conflicts.len(),
                                        onclick: move |_| selected_index.set(idx + 1),
                                        "Next"
                                    }
                                }
                            }

                            div { class: "flex items-center gap-2",
                                Button {
                                    class: "text-xs h-10 px-4 rounded-xl border border-emerald-500/40 bg-emerald-500/10 text-emerald-600 hover:bg-emerald-500/20 cursor-pointer font-semibold",
                                    disabled: is_resolving,
                                    onclick: {
                                        let u = props.active_user_id.clone();
                                        let w = props.workspace_id.clone();
                                        let cb = props.onresolvecomplete.clone();
                                        move |_| {
                                            let conflicts = conflicts_state.read().clone();
                                            let idx = *selected_index.read();
                                            if idx >= conflicts.len() { return; }
                                            let item = conflicts[idx].clone();
                                            is_resolving.set(true);
                                            status_msg.set(None);
                                            let u = u.clone();
                                            let w = w.clone();
                                            let cb = cb.clone();
                                            spawn(async move {
                                                if let Ok(_) = resolve_sync_conflict(u.clone(), w.clone(), item.table_name, item.record_id, "keep_local".to_string(), None).await {
                                                    status_msg.set(Some("Resolved using Local Device version!".to_string()));
                                                    cb.call(());
                                                    if let Ok(list) = get_sync_conflicts(w).await {
                                                        conflicts_state.set(list);
                                                    }
                                                }
                                                is_resolving.set(false);
                                            });
                                        }
                                    },
                                    LucideIcon { name: "check", class: "h-4 w-4 mr-1" }
                                    "Keep Local Version"
                                }

                                Button {
                                    class: "text-xs h-10 px-4 rounded-xl border border-primary/40 bg-primary/10 text-primary hover:bg-primary/20 cursor-pointer font-semibold",
                                    disabled: is_resolving,
                                    onclick: {
                                        let u = props.active_user_id.clone();
                                        let w = props.workspace_id.clone();
                                        let cb = props.onresolvecomplete.clone();
                                        move |_| {
                                            let conflicts = conflicts_state.read().clone();
                                            let idx = *selected_index.read();
                                            if idx >= conflicts.len() { return; }
                                            let item = conflicts[idx].clone();
                                            is_resolving.set(true);
                                            status_msg.set(None);
                                            let u = u.clone();
                                            let w = w.clone();
                                            let cb = cb.clone();
                                            spawn(async move {
                                                if let Ok(_) = resolve_sync_conflict(u.clone(), w.clone(), item.table_name, item.record_id, "keep_remote".to_string(), None).await {
                                                    status_msg.set(Some("Resolved using Server Cloud payload!".to_string()));
                                                    cb.call(());
                                                    if let Ok(list) = get_sync_conflicts(w).await {
                                                        conflicts_state.set(list);
                                                    }
                                                }
                                                is_resolving.set(false);
                                            });
                                        }
                                    },
                                    LucideIcon { name: "cloud-download", class: "h-4 w-4 mr-1" }
                                    "Keep Server Version"
                                }

                                Button {
                                    class: "text-xs h-10 px-5 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer font-bold",
                                    disabled: is_resolving,
                                    onclick: {
                                        let u = props.active_user_id.clone();
                                        let w = props.workspace_id.clone();
                                        let cb = props.onresolvecomplete.clone();
                                        move |_| {
                                            let conflicts = conflicts_state.read().clone();
                                            let idx = *selected_index.read();
                                            if idx >= conflicts.len() { return; }
                                            let item = conflicts[idx].clone();
                                            is_resolving.set(true);
                                            status_msg.set(None);
                                            let u = u.clone();
                                            let w = w.clone();
                                            let cb = cb.clone();
                                            spawn(async move {
                                                if let Ok(_) = resolve_sync_conflict(u.clone(), w.clone(), item.table_name, item.record_id, "crdt_merge".to_string(), None).await {
                                                    status_msg.set(Some("Conflict harmonized via CRDT merge engine!".to_string()));
                                                    cb.call(());
                                                    if let Ok(list) = get_sync_conflicts(w).await {
                                                        conflicts_state.set(list);
                                                    }
                                                }
                                                is_resolving.set(false);
                                            });
                                        }
                                    },
                                    LucideIcon { name: "sparkles", class: "h-4 w-4 mr-1" }
                                    "Auto-Merge CRDT"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
