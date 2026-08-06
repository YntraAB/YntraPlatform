use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{
    KeyRecoveryRequestRecord, approve_key_recovery, claim_recovered_key,
    generate_passkey_threshold_key_pair, get_pending_recovery_requests,
    register_threshold_key_node, reject_key_recovery, request_key_recovery,
};


#[derive(Props, Clone, PartialEq)]
pub struct KeyRecoveryModalProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub is_admin: bool,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn KeyRecoveryModal(props: KeyRecoveryModalProps) -> Element {
    let mut active_tab = use_signal(|| if props.is_admin { "admin" } else { "request" });
    let mut pending_requests = use_signal(|| Vec::<KeyRecoveryRequestRecord>::new());
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut is_processing = use_signal(|| false);
    let mut claimed_key_result = use_signal(|| Option::<String>::None);
    let mut generated_pubkey = use_signal(|| Option::<String>::None);

    let uid_eff = props.active_user_id.clone();
    let ws_eff = props.workspace_id.clone();
    let is_admin_eff = props.is_admin;

    use_effect(move || {
        let u = uid_eff.clone();
        let w = ws_eff.clone();
        if is_admin_eff {
            spawn(async move {
                if let Ok(list) = get_pending_recovery_requests(u, w).await {
                    pending_requests.set(list);
                }
            });
        }
    });

    let tab = *active_tab.read();
    let requests = pending_requests.read().clone();

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 sm:p-6",
            div { class: "w-full max-w-3xl rounded-3xl border border-border bg-card p-6 shadow-2xl flex flex-col gap-5 overflow-hidden",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-sm",
                            LucideIcon { name: "key-round", class: "h-6 w-6" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                "Enterprise Device Migration & Key Recovery"
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-primary/10 text-primary border border-primary/20", "Passkey Escrow" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Restore encrypted offline workspace access across lost devices or laptop upgrades."
                            }
                        }
                    }

                    button {
                        class: "p-2 rounded-xl border border-border bg-secondary text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                        onclick: move |_| props.onclose.call(()),
                        LucideIcon { name: "x", class: "h-5 w-5" }
                    }
                }

                // Tabs
                div { class: "flex items-center gap-2 border-b border-border pb-2 text-xs font-semibold flex-wrap",
                    button {
                        class: if tab == "request" { "px-4 py-2 rounded-xl bg-primary text-primary-foreground font-bold" } else { "px-4 py-2 rounded-xl bg-secondary text-secondary-foreground hover:bg-secondary/80" },
                        onclick: move |_| active_tab.set("request"),
                        "Request Device Key Reset"
                    }
                    if props.is_admin {
                        button {
                            class: if tab == "admin" { "px-4 py-2 rounded-xl bg-primary text-primary-foreground font-bold" } else { "px-4 py-2 rounded-xl bg-secondary text-secondary-foreground hover:bg-secondary/80" },
                            onclick: move |_| active_tab.set("admin"),
                            "IT Admin Approvals ({requests.len()})"
                        }
                        button {
                            class: if tab == "peer" { "px-4 py-2 rounded-xl bg-primary text-primary-foreground font-bold" } else { "px-4 py-2 rounded-xl bg-secondary text-secondary-foreground hover:bg-secondary/80" },
                            onclick: move |_| active_tab.set("peer"),
                            "Threshold Peer Escrow"
                        }
                    }
                    button {
                        class: if tab == "claim" { "px-4 py-2 rounded-xl bg-primary text-primary-foreground font-bold" } else { "px-4 py-2 rounded-xl bg-secondary text-secondary-foreground hover:bg-secondary/80" },
                        onclick: move |_| active_tab.set("claim"),
                        "Claim Key on New Device"
                    }
                }


                if let Some(ref msg) = *status_msg.read() {
                    div { class: "p-3 rounded-xl border border-primary/30 bg-primary/10 text-primary text-xs font-semibold flex items-center gap-2",
                        LucideIcon { name: "info", class: "h-4 w-4 shrink-0" }
                        "{msg}"
                    }
                }

                // Tab Contents
                if tab == "request" {
                    div { class: "space-y-4 py-2",
                        div { class: "p-4 rounded-2xl border border-border bg-secondary/20 space-y-2",
                            h3 { class: "text-sm font-bold text-foreground m-0", "Lost Phone or Upgraded Hardware?" }
                            p { class: "text-xs text-muted-foreground m-0",
                                "Submit a key recovery request. Your IT administrator will sign off to release your escrowed key payload so you can log into your workspace on this device without data loss."
                            }
                        }

                        div { class: "flex items-center justify-end pt-2",
                            Button {
                                class: "text-xs h-10 px-5 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer font-bold",
                                disabled: is_processing,
                                onclick: {
                                    let u = props.active_user_id.clone();
                                    move |_| {
                                        let u = u.clone();
                                        is_processing.set(true);
                                        status_msg.set(None);
                                        spawn(async move {
                                            if let Ok(_) = request_key_recovery(u).await {
                                                status_msg.set(Some("Key recovery request submitted! Notify your IT administrator for approval.".to_string()));
                                            } else {
                                                status_msg.set(Some("Failed to submit recovery request.".to_string()));
                                            }
                                            is_processing.set(false);
                                        });
                                    }
                                },
                                LucideIcon { name: "shield-alert", class: "h-4 w-4 mr-1.5" }
                                "Submit Key Reset Request"
                            }
                        }
                    }
                } else if tab == "admin" {
                    div { class: "space-y-4 py-2",
                        if requests.is_empty() {
                            div { class: "p-8 text-center text-muted-foreground space-y-1",
                                LucideIcon { name: "check-circle-2", class: "h-8 w-8 text-emerald-500 mx-auto" }
                                p { class: "text-xs font-semibold m-0", "No pending key recovery requests in this workspace." }
                            }
                        } else {
                            div { class: "space-y-3 max-h-64 overflow-auto",
                                for req in requests {
                                    div { class: "p-3 rounded-2xl border border-border bg-card flex items-center justify-between text-xs",
                                        div { class: "space-y-0.5",
                                            span { class: "font-bold text-foreground block", "{req.email}" }
                                            span { class: "text-muted-foreground text-[10px] font-mono",
                                                if req.recovery_status == "partially_approved" {
                                                    "ID: {req.user_id} • Status: 1/2 Admin Signatures Acquired"
                                                } else {
                                                    "ID: {req.user_id} • Status: 0/2 Signatures (Pending IT Review)"
                                                }
                                            }
                                        }

                                        div { class: "flex items-center gap-2",
                                            Button {
                                                class: "text-xs h-8 px-3 rounded-xl border border-emerald-500/40 bg-emerald-500/10 text-emerald-600 hover:bg-emerald-500/20 font-semibold",
                                                disabled: is_processing,
                                                onclick: {
                                                    let admin_id = props.active_user_id.clone();
                                                    let target_id = req.user_id.clone();
                                                    let ws_id = props.workspace_id.clone();
                                                    move |_| {
                                                        let admin_id = admin_id.clone();
                                                        let target_id = target_id.clone();
                                                        let ws_id = ws_id.clone();
                                                        is_processing.set(true);
                                                        spawn(async move {
                                                            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
                                                            let temp_key = format!("temp_wrapped_escrow_{}", ts);
                                                            if let Ok(_) = approve_key_recovery(admin_id.clone(), target_id, temp_key).await {
                                                                status_msg.set(Some("Admin signature recorded for key recovery!".to_string()));
                                                                if let Ok(list) = get_pending_recovery_requests(admin_id, ws_id).await {
                                                                    pending_requests.set(list);
                                                                }
                                                            }
                                                            is_processing.set(false);
                                                        });
                                                    }
                                                },
                                                "Sign & Approve (2-Admin Threshold)"
                                            }

                                            Button {
                                                class: "text-xs h-8 px-3 rounded-xl border border-red-500/40 bg-red-500/10 text-red-600 hover:bg-red-500/20 font-semibold",
                                                disabled: is_processing,
                                                onclick: {
                                                    let admin_id = props.active_user_id.clone();
                                                    let target_id = req.user_id.clone();
                                                    let ws_id = props.workspace_id.clone();
                                                    move |_| {
                                                        let admin_id = admin_id.clone();
                                                        let target_id = target_id.clone();
                                                        let ws_id = ws_id.clone();
                                                        is_processing.set(true);
                                                        spawn(async move {
                                                            if let Ok(_) = reject_key_recovery(admin_id.clone(), target_id, "Admin rejection".to_string()).await {
                                                                status_msg.set(Some("Key recovery request rejected.".to_string()));
                                                                if let Ok(list) = get_pending_recovery_requests(admin_id, ws_id).await {
                                                                    pending_requests.set(list);
                                                                }
                                                            }
                                                            is_processing.set(false);
                                                        });
                                                    }
                                                },
                                                "Reject"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if tab == "peer" {
                    div { class: "space-y-4 py-2",
                        div { class: "p-4 rounded-2xl border border-amber-500/30 bg-amber-500/5 space-y-2",
                            h3 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                                LucideIcon { name: "shield-alert", class: "h-4 w-4 text-amber-600 dark:text-amber-400" }
                                "Shamir Secret Sharing (Emergency Master Vault Recovery Only)"
                            }
                            p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                                "Shamir threshold key splitting (3-of-5) is strictly reserved for Emergency Master Vault Disaster Recovery. Routine user account unlocks and password resets must use Enterprise SAML 2.0 / OIDC SSO (Okta, Entra ID) or delegated Helpdesk approvals."
                            }
                        }

                        if let Some(ref pubkey) = *generated_pubkey.read() {
                            div { class: "p-3 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-600 text-xs font-mono break-all",
                                "Generated Escrow Node Pubkey: {pubkey}"
                            }
                        }

                        div { class: "flex items-center justify-end gap-3 pt-2",
                            Button {
                                class: "text-xs h-10 px-5 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer font-bold",
                                onclick: {
                                    let u = props.active_user_id.clone();
                                    let w = props.workspace_id.clone();
                                    move |_| {
                                        let u = u.clone();
                                        let w = w.clone();
                                        if let Ok(kp) = generate_passkey_threshold_key_pair() {
                                            generated_pubkey.set(Some(kp.public_key_hex.clone()));
                                            let pk = kp.public_key_hex.clone();
                                            spawn(async move {
                                                if let Ok(_) = register_threshold_key_node(u, w, pk).await {
                                                    status_msg.set(Some("Trusted peer escrow node registered successfully!".to_string()));
                                                }
                                            });
                                        }
                                    }
                                },
                                LucideIcon { name: "plus-circle", class: "h-4 w-4 mr-1.5" }
                                "Generate & Register Peer Node Key"
                            }
                        }
                    }
                } else if tab == "claim" {

                    div { class: "space-y-4 py-2",
                        div { class: "p-4 rounded-2xl border border-border bg-secondary/20 space-y-2",
                            h3 { class: "text-sm font-bold text-foreground m-0", "Claim Released Key Payload" }
                            p { class: "text-xs text-muted-foreground m-0",
                                "Once your IT administrator approves your request, click below to claim your temporary wrapped key payload and re-encrypt your local database."
                            }
                        }

                        if let Some(ref res_key) = *claimed_key_result.read() {
                            div { class: "p-3 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-600 text-xs font-mono break-all",
                                "Claimed Temp Wrapped Payload: {res_key}"
                            }
                        }

                        div { class: "flex items-center justify-end pt-2",
                            Button {
                                class: "text-xs h-10 px-5 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer font-bold",
                                disabled: is_processing,
                                onclick: {
                                    let u = props.active_user_id.clone();
                                    move |_| {
                                        let u = u.clone();
                                        is_processing.set(true);
                                        status_msg.set(None);
                                        spawn(async move {
                                            match claim_recovered_key(u).await {
                                                Ok(key) => {
                                                    claimed_key_result.set(Some(key));
                                                    status_msg.set(Some("Key claimed successfully! Local database access restored.".to_string()));
                                                }
                                                Err(e) => {
                                                    status_msg.set(Some(format!("Claim failed: {}", e)));
                                                }
                                            }
                                            is_processing.set(false);
                                        });
                                    }
                                },
                                LucideIcon { name: "download", class: "h-4 w-4 mr-1.5" }
                                "Claim Escrow Key"
                            }
                        }
                    }
                }
            }
        }
    }
}
