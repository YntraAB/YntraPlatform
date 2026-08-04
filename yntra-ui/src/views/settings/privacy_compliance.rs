use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{delete_user_account, export_user_personal_data, set_telemetry_opt_out, WorkspaceUser};

#[derive(Props, Clone)]
pub struct PrivacyComplianceCardProps {
    pub active_user: WorkspaceUser,
    pub account_preferences: Signal<String>,
    pub locale: String,
}

impl PartialEq for PrivacyComplianceCardProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn PrivacyComplianceCard(props: PrivacyComplianceCardProps) -> Element {
    let active_user = props.active_user.clone();
    let account_preferences = props.account_preferences;
    let locale = props.locale.clone();

    let mut is_exporting = use_signal(|| false);
    let mut export_output = use_signal(|| Option::<String>::None);
    let mut is_deleting = use_signal(|| false);
    let mut show_delete_confirm = use_signal(|| false);
    let mut delete_status = use_signal(|| Option::<String>::None);

    let mut telemetry_opt_out = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("telemetry_opt_out")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    });

    let handle_export = {
        let user_id = active_user.id.clone();
        move |_| {
            is_exporting.set(true);
            let uid = user_id.clone();
            spawn(async move {
                match export_user_personal_data(uid).await {
                    Ok(json_data) => {
                        export_output.set(Some(json_data));
                    }
                    Err(e) => {
                        export_output.set(Some(format!("Export error: {}", e)));
                    }
                }
                is_exporting.set(false);
            });
        }
    };

    let handle_delete = {
        let user_id = active_user.id.clone();
        move |_| {
            is_deleting.set(true);
            let uid = user_id.clone();
            spawn(async move {
                match delete_user_account(uid.clone(), uid).await {
                    Ok(_) => {
                        delete_status.set(Some("Account and data successfully deleted.".to_string()));
                    }
                    Err(e) => {
                        delete_status.set(Some(format!("Deletion failed: {}", e)));
                    }
                }
                is_deleting.set(false);
            });
        }
    };

    let mut handle_toggle_telemetry = {
        let user_id = active_user.id.clone();
        move |opt_out: bool| {
            telemetry_opt_out.set(opt_out);
            let uid = user_id.clone();
            spawn(async move {
                let _ = set_telemetry_opt_out(uid, opt_out).await;
            });
        }
    };

    rsx! {
        components::Card { class: "border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-sm",
            components::CardHeader {
                div { class: "flex items-center gap-3",
                    div { class: "rounded-lg bg-primary/10 p-2 text-primary",
                        components::LucideIcon { name: "shield-check", class: "h-5 w-5" }
                    }
                    div {
                        components::CardTitle { class: "text-lg", "{t(\"settings-privacy-title\", &locale)}" }
                        components::CardDescription { "{t(\"settings-privacy-desc\", &locale)}" }
                    }
                }
            }
            components::CardContent { class: "space-y-6",
                
                // 1. Data Export Section (GDPR Art. 20)
                div { class: "flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border-b border-border/40 pb-4",
                    div { class: "space-y-1 flex-1",
                        h4 { class: "text-sm font-semibold text-foreground m-0", "{t(\"settings-gdpr-export-title\", &locale)}" }
                        p { class: "text-xs text-muted-foreground m-0", "{t(\"settings-gdpr-export-desc\", &locale)}" }
                    }
                    components::Button {
                        variant: components::ButtonVariant::Outline,
                        disabled: *is_exporting.read(),
                        onclick: handle_export,
                        components::LucideIcon { name: "download", class: "mr-2 h-4 w-4" }
                        span { if *is_exporting.read() { "Exporting..." } else { "{t(\"settings-gdpr-export-button\", &locale)}" } }
                    }
                }

                if let Some(json_payload) = export_output.read().as_ref() {
                    div { class: "rounded-xl border border-primary/20 bg-muted/40 p-3 space-y-2",
                        div { class: "flex justify-between items-center",
                            span { class: "text-[11px] font-bold text-primary uppercase tracking-wider", "GDPR Personal Data Archive Ready" }
                            button {
                                class: "text-xs text-muted-foreground hover:text-foreground underline",
                                onclick: move |_| export_output.set(None),
                                "Dismiss"
                            }
                        }
                        textarea {
                            class: "w-full h-32 font-mono text-[10px] bg-background/80 p-2 rounded border border-border/50 focus:outline-none",
                            readonly: true,
                            value: "{json_payload}"
                        }
                    }
                }

                // 2. Telemetry Opt-Out Section
                div { class: "flex items-center justify-between border-b border-border/40 pb-4",
                    div { class: "space-y-1 pr-4",
                        h4 { class: "text-sm font-semibold text-foreground m-0", "{t(\"settings-telemetry-optout-title\", &locale)}" }
                        p { class: "text-xs text-muted-foreground m-0", "{t(\"settings-telemetry-optout-desc\", &locale)}" }
                    }
                    input {
                        r#type: "checkbox",
                        class: "h-5 w-5 rounded border-border text-primary focus:ring-primary cursor-pointer",
                        checked: *telemetry_opt_out.read(),
                        onchange: move |e: Event<FormData>| handle_toggle_telemetry(e.value() == "true")
                    }
                }

                // 3. Legal Documents Links
                div { class: "flex flex-wrap gap-3 border-b border-border/40 pb-4",
                    a {
                        href: "/TERMS.md",
                        target: "_blank",
                        class: "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border/50 bg-secondary/30 text-xs font-semibold hover:bg-secondary/60 transition-all",
                        components::LucideIcon { name: "file-text", class: "h-3.5 w-3.5" }
                        "{t(\"settings-legal-tos-button\", &locale)}"
                    }
                    a {
                        href: "/PRIVACY.md",
                        target: "_blank",
                        class: "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border/50 bg-secondary/30 text-xs font-semibold hover:bg-secondary/60 transition-all",
                        components::LucideIcon { name: "shield", class: "h-3.5 w-3.5" }
                        "{t(\"settings-legal-privacy-button\", &locale)}"
                    }
                }

                // 4. Account Erasure Section (GDPR Art. 17)
                div { class: "flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 pt-2",
                    div { class: "space-y-1 flex-1",
                        h4 { class: "text-sm font-semibold text-destructive m-0", "{t(\"settings-gdpr-delete-title\", &locale)}" }
                        p { class: "text-xs text-muted-foreground m-0", "{t(\"settings-gdpr-delete-desc\", &locale)}" }
                    }
                    components::Button {
                        variant: components::ButtonVariant::Destructive,
                        onclick: move |_| show_delete_confirm.set(true),
                        components::LucideIcon { name: "trash-2", class: "mr-2 h-4 w-4" }
                        span { "{t(\"settings-gdpr-delete-button\", &locale)}" }
                    }
                }

                if *show_delete_confirm.read() {
                    div { class: "rounded-xl border border-destructive/40 bg-destructive/5 p-4 space-y-3",
                        p { class: "text-xs font-bold text-destructive m-0", "Confirm Permanent Account Deletion" }
                        p { class: "text-[11px] text-muted-foreground m-0", "This will erase all your profile info, user signatures, and local credentials. Are you sure?" }
                        div { class: "flex items-center gap-2 pt-1",
                            components::Button {
                                variant: components::ButtonVariant::Destructive,
                                disabled: *is_deleting.read(),
                                onclick: handle_delete,
                                if *is_deleting.read() { "Deleting..." } else { "Confirm Permanent Erasure" }
                            }
                            components::Button {
                                variant: components::ButtonVariant::Outline,
                                onclick: move |_| show_delete_confirm.set(false),
                                "Cancel"
                            }
                        }
                    }
                }

                if let Some(msg) = delete_status.read().as_ref() {
                    p { class: "text-xs font-bold text-emerald-500 m-0", "{msg}" }
                }
            }
        }
    }
}
