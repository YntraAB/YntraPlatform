use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{
    AiActionApprovalItem, get_pending_ai_action_approvals, review_ai_action_approval,
};

#[derive(Props, Clone, PartialEq)]
pub struct AiApprovalModalProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn AiApprovalModal(props: AiApprovalModalProps) -> Element {
    let mut approvals_state = use_signal(|| Vec::<AiActionApprovalItem>::new());
    let mut is_reviewing = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| true);

    let uid_eff = props.active_user_id.clone();
    let ws_eff = props.workspace_id.clone();

    use_effect(move || {
        let u = uid_eff.clone();
        let w = ws_eff.clone();
        spawn(async move {
            is_loading.set(true);
            if let Ok(list) = get_pending_ai_action_approvals(u, w).await {
                approvals_state.set(list);
            }
            is_loading.set(false);
        });
    });

    let items = approvals_state.read().clone();

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 sm:p-6",
            div { class: "w-full max-w-4xl h-[80vh] rounded-3xl border border-border bg-card p-6 shadow-2xl flex flex-col gap-5 overflow-hidden",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-purple-500/10 flex items-center justify-center text-purple-500 shadow-sm",
                            LucideIcon { name: "bot", class: "h-6 w-6" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                "Human-In-The-Loop AI Action Approvals"
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-purple-500/10 text-purple-600 border border-purple-500/20", "Explainable AI" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Review automated AI guardrail flags, confidence scores, and action explainability rationales"
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

                // Main List
                div { class: "flex-1 overflow-y-auto space-y-4 pr-1",
                    if *is_loading.read() {
                        div { class: "p-12 text-center text-muted-foreground italic", "Loading pending AI action proposals..." }
                    } else if items.is_empty() {
                        div { class: "p-12 text-center text-muted-foreground space-y-2",
                            LucideIcon { name: "sparkles", class: "h-10 w-10 text-purple-500 mx-auto" }
                            h3 { class: "text-base font-bold text-foreground m-0", "Zero AI Approvals Pending" }
                            p { class: "text-xs m-0", "All automated guardrails and shift proposals have been processed." }
                        }
                    } else {
                        for item in items.iter() {
                            {
                                let item_id = item.id.clone();
                                let risk_pct = (item.risk_score * 100.0) as u32;
                                let is_high_risk = item.risk_score > 0.7;

                                rsx! {
                                    div { key: "{item.id}", class: "rounded-2xl border border-border bg-background p-5 space-y-4 shadow-sm hover:border-primary/40 transition-all",
                                        // Header Row
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-2.5",
                                                span { class: "px-2.5 py-0.5 rounded-full text-xs font-bold bg-primary/10 text-primary border border-primary/20",
                                                    "{item.proposed_action_type}"
                                                }
                                                span { class: "text-xs text-muted-foreground font-mono", "Target User: {item.target_resource_id}" }
                                            }

                                            div { class: "flex items-center gap-2",
                                                span { class: "text-[11px] font-semibold text-muted-foreground", "Risk Score:" }
                                                span { class: format!(
                                                    "px-2.5 py-0.5 rounded-full text-[10px] font-bold border {}",
                                                    if is_high_risk { "bg-destructive/10 text-destructive border-destructive/30" } else { "bg-amber-500/10 text-amber-600 border-amber-500/30" }
                                                ),
                                                    "{risk_pct}% Risk"
                                                }
                                            }
                                        }

                                        // Action Explainability Card
                                        div { class: "p-3.5 rounded-xl border border-purple-500/20 bg-purple-500/5 space-y-1",
                                            div { class: "flex items-center gap-1.5 text-xs font-bold text-purple-600",
                                                LucideIcon { name: "help-circle", class: "h-4 w-4" }
                                                "Action Explainability Rationale:"
                                            }
                                            p { class: "text-xs text-foreground m-0 leading-relaxed font-medium",
                                                "{item.explainability_rationale}"
                                            }
                                        }

                                        // Manager Actions
                                        div { class: "flex items-center justify-end gap-3 pt-2 border-t border-border/40",
                                            Button {
                                                class: "text-xs h-9 px-4 rounded-xl border border-destructive/40 bg-destructive/10 text-destructive hover:bg-destructive/20 cursor-pointer font-semibold",
                                                disabled: is_reviewing,
                                                onclick: {
                                                    let id = item_id.clone();
                                                    let u = props.active_user_id.clone();
                                                    let w = props.workspace_id.clone();
                                                    move |_| {
                                                        is_reviewing.set(true);
                                                        status_msg.set(None);
                                                        let id = id.clone();
                                                        let u = u.clone();
                                                        let w = w.clone();
                                                        spawn(async move {
                                                            match review_ai_action_approval(u.clone(), w.clone(), id, false, None).await {
                                                                Ok(_) => {
                                                                    status_msg.set(Some("AI Action Rejected. Observers notified.".to_string()));
                                                                    if let Ok(list) = get_pending_ai_action_approvals(u, w).await {
                                                                        approvals_state.set(list);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    status_msg.set(Some(format!("Review error: {}", e)));
                                                                }
                                                            }
                                                            is_reviewing.set(false);
                                                        });
                                                    }
                                                },
                                                LucideIcon { name: "x-circle", class: "h-3.5 w-3.5 mr-1" }
                                                "Reject AI Action"
                                            }

                                            Button {
                                                class: "text-xs h-9 px-5 rounded-xl bg-emerald-600 text-white hover:bg-emerald-500 shadow-sm cursor-pointer font-bold",
                                                disabled: is_reviewing,
                                                onclick: {
                                                    let id = item_id.clone();
                                                    let u = props.active_user_id.clone();
                                                    let w = props.workspace_id.clone();
                                                    move |_| {
                                                        is_reviewing.set(true);
                                                        status_msg.set(None);
                                                        let id = id.clone();
                                                        let u = u.clone();
                                                        let w = w.clone();
                                                        spawn(async move {
                                                            match review_ai_action_approval(u.clone(), w.clone(), id, true, None).await {
                                                                Ok(_) => {
                                                                    status_msg.set(Some("AI Action Approved. Observers notified.".to_string()));
                                                                    if let Ok(list) = get_pending_ai_action_approvals(u, w).await {
                                                                        approvals_state.set(list);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    status_msg.set(Some(format!("Review error: {}", e)));
                                                                }
                                                            }
                                                            is_reviewing.set(false);
                                                        });
                                                    }
                                                },
                                                LucideIcon { name: "check-circle", class: "h-3.5 w-3.5 mr-1" }
                                                "Approve AI Action"
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
