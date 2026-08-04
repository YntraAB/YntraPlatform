use crate::components::{Button, Dialog, LucideIcon, PasskeyCard};
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{Workspace, ZkCryptoTrust, delete_user_account, export_user_personal_data};

#[derive(Props, Clone, PartialEq)]
pub struct PrivacyComplianceViewProps {
    pub active_user_id: String,
    pub workspace: Workspace,
    pub locale: String,
}

#[component]
pub fn PrivacyComplianceView(props: PrivacyComplianceViewProps) -> Element {
    let loc = props.locale.as_str();

    let mut show_export_modal = use_signal(|| false);
    let mut show_delete_modal = use_signal(|| false);
    let mut export_json_content = use_signal(|| Option::<String>::None);
    let mut is_exporting = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);
    let mut delete_confirm_input = use_signal(|| String::new());

    // ZKP verification test state
    let mut zkp_verified = use_signal(|| Option::<bool>::None);

    let uid = props.active_user_id.clone();
    let ws_id = props.workspace.id.clone();

    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background px-8 py-6 space-y-8 overflow-y-auto",
            // Page Header
            div { class: "border-b border-border pb-4",
                div { class: "flex items-center gap-3",
                    div { class: "h-12 w-12 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-sm",
                        LucideIcon { name: "shield-alert", class: "h-7 w-7" }
                    }
                    div {
                        h2 { class: "text-xl font-bold text-foreground flex items-center gap-2.5 m-0",
                            "Trust, Passkeys & Compliance Dashboard"
                            span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-primary/10 text-primary border border-primary/20",
                                "GDPR / CCPA Ready"
                            }
                        }
                        p { class: "text-xs text-muted-foreground m-0 mt-1",
                            "{t(\"settings-privacy-desc\", loc)}"
                        }
                    }
                }
            }

            // Section 1: Hardware Passkeys & Envelope Encryption
            PasskeyCard {
                active_user_id: props.active_user_id.clone(),
                workspace_id: props.workspace.id.clone(),
                locale: props.locale.clone(),
            }

            // Section 2: GDPR / CCPA Data Portability & Account Erasure Grid
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                // Data Portability (GDPR Art. 20) Card
                div { class: "rounded-3xl border border-border bg-card p-6 shadow-sm flex flex-col justify-between space-y-5",
                    div { class: "space-y-3",
                        div { class: "flex items-center gap-3",
                            div { class: "h-10 w-10 rounded-2xl bg-blue-500/10 text-blue-500 flex items-center justify-center font-bold",
                                LucideIcon { name: "download", class: "h-5 w-5" }
                            }
                            div {
                                h3 { class: "text-sm font-bold text-foreground m-0", "{t(\"settings-gdpr-export-title\", loc)}" }
                                span { class: "text-[10px] font-semibold text-blue-600 uppercase tracking-wider", "Article 20 Data Portability" }
                            }
                        }
                        p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                            "{t(\"settings-gdpr-export-desc\", loc)}"
                        }
                    }

                    div { class: "pt-3 border-t border-border/40 flex items-center justify-between",
                        span { class: "text-[11px] font-mono text-muted-foreground", "Schema: 1.0-GDPR-CCPA" }
                        Button {
                            class: "text-xs h-9 px-4 rounded-xl bg-primary text-primary-foreground shadow-sm cursor-pointer",
                            disabled: *is_exporting.read(),
                            onclick: {
                                let req_uid = props.active_user_id.clone();
                                move |_| {
                                    is_exporting.set(true);
                                    let req_uid = req_uid.clone();
                                    spawn(async move {
                                        if let Ok(json_str) = export_user_personal_data(req_uid).await {
                                            export_json_content.set(Some(json_str));
                                            show_export_modal.set(true);
                                        }
                                        is_exporting.set(false);
                                    });
                                }
                            },
                            if *is_exporting.read() {
                                LucideIcon { name: "refresh-cw", class: "h-4 w-4 animate-spin mr-1.5" }
                                "Exporting..."
                            } else {
                                LucideIcon { name: "file-json", class: "h-4 w-4 mr-1.5" }
                                "{t(\"settings-gdpr-export-button\", loc)}"
                            }
                        }
                    }
                }

                // Account Erasure (GDPR Art. 17) Card
                div { class: "rounded-3xl border border-red-500/20 bg-red-500/5 p-6 shadow-sm flex flex-col justify-between space-y-5",
                    div { class: "space-y-3",
                        div { class: "flex items-center gap-3",
                            div { class: "h-10 w-10 rounded-2xl bg-red-500/10 text-red-500 flex items-center justify-center font-bold",
                                LucideIcon { name: "user-x", class: "h-5 w-5" }
                            }
                            div {
                                h3 { class: "text-sm font-bold text-foreground m-0", "{t(\"settings-gdpr-delete-title\", loc)}" }
                                span { class: "text-[10px] font-semibold text-red-600 uppercase tracking-wider", "Article 17 Right to be Forgotten" }
                            }
                        }
                        p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                            "{t(\"settings-gdpr-delete-desc\", loc)}"
                        }
                    }

                    div { class: "pt-3 border-t border-red-500/20 flex items-center justify-between",
                        span { class: "text-[11px] font-bold text-red-600", "Irreversible Action" }
                        Button {
                            class: "text-xs h-9 px-4 rounded-xl bg-red-600 text-white hover:bg-red-700 shadow-sm cursor-pointer",
                            onclick: move |_| show_delete_modal.set(true),
                            LucideIcon { name: "trash-2", class: "h-4 w-4 mr-1.5" }
                            "{t(\"settings-gdpr-delete-button\", loc)}"
                        }
                    }
                }
            }

            // Section 3: Zero-Knowledge Proof (ZKP) Inspector
            div { class: "rounded-3xl border border-border bg-card p-6 shadow-sm space-y-4",
                div { class: "flex items-center justify-between border-b border-border pb-3",
                    div { class: "flex items-center gap-2.5",
                        LucideIcon { name: "cpu", class: "h-5 w-5 text-primary" }
                        h3 { class: "text-sm font-bold text-foreground m-0", "Zero-Knowledge Write Authorization Inspector" }
                    }
                    Button {
                        class: "text-xs h-8 px-3 rounded-xl border border-border bg-secondary text-secondary-foreground cursor-pointer",
                        onclick: move |_| {
                            let trust = ZkCryptoTrust::new();
                            let proof_res = trust.generate_role_proof("seed-passkey-123".to_string(), uid.clone(), "admin".to_string());
                            if let Ok(proof) = proof_res {
                                let verified = trust.verify_proof(proof, uid.clone(), "admin".to_string(), "00".to_string());
                                zkp_verified.set(Some(verified));
                            } else {
                                zkp_verified.set(Some(false));
                            }
                        },
                        LucideIcon { name: "check-circle-2", class: "h-3.5 w-3.5 mr-1" }
                        "Verify ZKP Proof"
                    }
                }

                p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                    "The local node generates non-interactive Zero-Knowledge proofs for table transactions. Remote sync nodes verify authorization without accessing your role or identity secrets."
                }

                if let Some(is_valid) = *zkp_verified.read() {
                    div { class: format!(
                        "p-3 rounded-xl text-xs font-semibold flex items-center gap-2 border {}",
                        if is_valid { "bg-emerald-500/10 border-emerald-500/30 text-emerald-600" } else { "bg-red-500/10 border-red-500/30 text-red-600" }
                    ),
                        LucideIcon { name: if is_valid { "check-check" } else { "alert-circle" }, class: "h-4 w-4" }
                        "ZKP Verification Test: Proof generated and verified using ZkCryptoTrust."
                    }
                }
            }

            // Modal 1: Data Export JSON Preview Modal
            if *show_export_modal.read() {
                {
                    let json_preview_str = export_json_content.read().clone().unwrap_or_else(|| "{}".to_string());
                    rsx! {
                        Dialog {
                            open: *show_export_modal.read(),
                            title: "GDPR Art. 20 Personal Data Export Bundle".to_string(),
                            max_width: "650px".to_string(),
                            onclose: move |_| show_export_modal.set(false),

                            div { class: "space-y-4",
                                p { class: "text-xs text-muted-foreground m-0",
                                    "Machine-readable JSON archive containing user profile metadata, cryptographic signatures, and privacy parameters."
                                }

                                div { class: "max-h-80 overflow-y-auto p-4 rounded-2xl bg-muted font-mono text-[11px] text-foreground border border-border whitespace-pre-wrap break-all",
                                    "{json_preview_str}"
                                }

                                div { class: "flex items-center justify-end gap-3 pt-2",
                                    Button {
                                        class: "text-xs h-9 px-4 rounded-xl border border-border bg-secondary text-secondary-foreground",
                                        onclick: move |_| show_export_modal.set(false),
                                        "Close"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Modal 2: Right to be Forgotten Double Confirmation Modal
            if *show_delete_modal.read() {
                Dialog {
                    open: *show_delete_modal.read(),
                    title: "Confirm Permanent Account & Data Erasure".to_string(),
                    max_width: "500px".to_string(),
                    onclose: move |_| show_delete_modal.set(false),

                    div { class: "space-y-4",
                        div { class: "p-4 rounded-2xl bg-red-500/10 border border-red-500/30 text-red-600 space-y-2",
                            div { class: "flex items-center gap-2 font-bold text-xs",
                                LucideIcon { name: "alert-triangle", class: "h-4 w-4 shrink-0" }
                                "WARNING: Permanent Irreversible Erasure"
                            }
                            p { class: "text-xs m-0 leading-relaxed text-red-600/90",
                                "This will permanently delete your user profile, cryptographic credentials, signatures, and local SQLite data. Type 'DELETE' below to confirm."
                            }
                        }

                        div { class: "space-y-1.5",
                            label { class: "text-xs font-semibold text-foreground", "Type 'DELETE' to confirm" }
                            input {
                                class: "w-full h-9 px-3 rounded-xl border border-border bg-background text-xs font-mono text-foreground focus:outline-hidden focus:border-red-500",
                                placeholder: "DELETE",
                                value: "{delete_confirm_input.read()}",
                                oninput: move |e| delete_confirm_input.set(e.value()),
                            }
                        }

                        div { class: "flex items-center justify-end gap-3 pt-2",
                            Button {
                                class: "text-xs h-9 px-4 rounded-xl border border-border bg-secondary text-secondary-foreground",
                                onclick: move |_| show_delete_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "text-xs h-9 px-4 rounded-xl bg-red-600 text-white hover:bg-red-700 shadow-sm cursor-pointer",
                                disabled: delete_confirm_input.read().trim() != "DELETE" || *is_deleting.read(),
                                onclick: {
                                    let req_uid = props.active_user_id.clone();
                                    move |_| {
                                        is_deleting.set(true);
                                        let req_uid = req_uid.clone();
                                        let target_uid = req_uid.clone();
                                        spawn(async move {
                                            let _ = delete_user_account(req_uid, target_uid).await;
                                            is_deleting.set(false);
                                            show_delete_modal.set(false);
                                        });
                                    }
                                },
                                if *is_deleting.read() { "Erasing Account..." } else { "Confirm Irreversible Erasure" }
                            }
                        }
                    }
                }
            }
        }
    }
}
