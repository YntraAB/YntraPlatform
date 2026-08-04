use crate::components;
use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct OnboardingWizardProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub db_trigger: Signal<u32>,
    pub locale: String,
    pub on_complete: EventHandler<()>,
}

#[component]
pub fn OnboardingWizardView(props: OnboardingWizardProps) -> Element {
    let mut step = use_signal(|| 1);

    let mut name = use_signal(|| props.workspace.name.clone());
    let mut org_type = use_signal(|| "care".to_string());
    let mut brand_color = use_signal(|| props.workspace.brand_color.clone());

    // Module Toggles
    let mut module_messaging = use_signal(|| true);
    let mut module_scheduling = use_signal(|| true);
    let mut module_notes = use_signal(|| true);
    let mut module_time = use_signal(|| true);
    let mut module_reporting = use_signal(|| true);
    let mut module_care = use_signal(|| true);
    let mut module_dispatch = use_signal(|| false);

    // Initial Invites
    let mut invite_email = use_signal(String::new);
    let mut invite_role = use_signal(|| "member".to_string());
    let mut invite_list = use_signal(Vec::<(String, String)>::new);

    // Saving & State
    let mut is_saving = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let active_user_id = props.active_user.id.clone();
    let workspace_id = props.workspace.id.clone();
    let db_trigger = props.db_trigger;
    let on_complete = props.on_complete;
    let _locale = props.locale.clone();

    let add_invite = move |_| {
        let e = invite_email.read().trim().to_string();
        let r = invite_role.read().clone();
        if !e.is_empty() && e.contains('@') {
            invite_list.write().push((e, r));
            invite_email.set(String::new());
        }
    };

    let mut remove_invite = move |idx: usize| {
        invite_list.write().remove(idx);
    };

    let handle_finish = move |_| {
        is_saving.set(true);
        error_msg.set(None);

        let uid = active_user_id.clone();
        let wsid = workspace_id.clone();

        let modules_obj = serde_json::json!({
            "messaging": *module_messaging.read(),
            "scheduling": *module_scheduling.read(),
            "notes": *module_notes.read(),
            "time": *module_time.read(),
            "reporting": *module_reporting.read(),
            "assistance": *module_care.read(),
            "dispatch": *module_dispatch.read(),
            "directory": true
        });

        let payload = serde_json::json!({
            "name": name.read().clone(),
            "org_type": org_type.read().clone(),
            "brand_color": brand_color.read().clone(),
            "modules_active": modules_obj
        });

        let invites = invite_list.read().clone();
        let mut trigger = db_trigger;

        spawn(async move {
            match yntra_core::complete_workspace_onboarding(
                uid.clone(),
                wsid.clone(),
                payload.to_string(),
            )
            .await
            {
                Ok(_) => {
                    // Send initial invitations if any
                    for (inv_e, inv_r) in invites {
                        let _ = yntra_core::create_workspace_invitation(
                            uid.clone(),
                            wsid.clone(),
                            inv_e.clone(),
                            inv_e,
                            inv_r,
                        )
                        .await;
                    }

                    let trig_val = *trigger.read();
                    trigger.set(trig_val + 1);
                    is_saving.set(false);
                    status_msg.set(Some(
                        "Workspace onboarding successfully completed!".to_string(),
                    ));
                    on_complete.call(());
                }
                Err(e) => {
                    is_saving.set(false);
                    error_msg.set(Some(format!("Onboarding failed: {}", e)));
                }
            }
        });
    };

    rsx! {
        div { class: "max-w-4xl mx-auto p-6 space-y-8 animate-in fade-in duration-300",
            // Wizard Header
            div { class: "border-b border-border/40 pb-6 text-center space-y-2",
                div { class: "inline-flex items-center gap-2 px-3 py-1 rounded-full bg-primary/10 text-primary text-xs font-semibold uppercase tracking-wider",
                    components::LucideIcon { name: "sparkles", class: "h-3.5 w-3.5" }
                    "First-Time Workspace Onboarding"
                }
                h1 { class: "text-3xl font-extrabold text-foreground tracking-tight", "Welcome to Yntra Platform" }
                p { class: "text-sm text-muted-foreground max-w-xl mx-auto",
                    "Follow this guided wizard to configure your workspace details, activate functional modules, send team invitations, and establish security controls."
                }
            }

            // Step Progress Bar
            div { class: "grid grid-cols-5 gap-2 border border-border/40 rounded-xl p-3 bg-card/50 backdrop-blur-sm shadow-sm",
                for s in 1..=5 {
                    {
                        let is_active = *step.read() == s;
                        let is_completed = *step.read() > s;
                        let step_title = match s {
                            1 => "1. General",
                            2 => "2. Modules",
                            3 => "3. Invites",
                            4 => "4. Security",
                            _ => "5. Finish"
                        };
                        rsx! {
                            button {
                                key: "{s}",
                                class: format!(
                                    "flex flex-col items-center justify-center p-2 rounded-lg text-xs font-medium transition-all duration-200 {}",
                                    if is_active {
                                        "bg-primary text-primary-foreground font-semibold shadow-md scale-102"
                                    } else if is_completed {
                                        "bg-primary/20 text-primary font-medium"
                                    } else {
                                        "bg-muted/40 text-muted-foreground hover:bg-muted/60"
                                    }
                                ),
                                onclick: move |_| step.set(s),
                                span { "{step_title}" }
                            }
                        }
                    }
                }
            }

            // Status & Errors
            if let Some(err) = error_msg.read().as_ref() {
                div { class: "p-4 rounded-xl bg-red-500/10 border border-red-500/30 text-red-500 text-sm font-medium flex items-center gap-3 animate-in fade-in",
                    components::LucideIcon { name: "alert-circle", class: "h-5 w-5 shrink-0" }
                    span { "{err}" }
                }
            }

            if let Some(msg) = status_msg.read().as_ref() {
                div { class: "p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-500 text-sm font-medium flex items-center gap-3 animate-in fade-in",
                    components::LucideIcon { name: "check-circle-2", class: "h-5 w-5 shrink-0" }
                    span { "{msg}" }
                }
            }

            // Wizard Step Contents
            match *step.read() {
                1 => rsx! {
                    div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-6 animate-in fade-in duration-200",
                        div { class: "border-b border-border/30 pb-3",
                            h2 { class: "text-lg font-bold text-foreground", "Step 1: Workspace Identity" }
                            p { class: "text-xs text-muted-foreground", "Set your organization's display name, business type, and primary theme color." }
                        }

                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                            div { class: "space-y-2",
                                label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Workspace Name" }
                                input {
                                    class: "w-full px-4 py-2.5 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                                    value: "{name.read()}",
                                    oninput: move |e| name.set(e.value())
                                }
                            }

                            div { class: "space-y-2",
                                label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Organization Type" }
                                select {
                                    class: "w-full px-4 py-2.5 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                                    value: "{org_type.read()}",
                                    onchange: move |e| org_type.set(e.value()),
                                    option { value: "care", "Healthcare & Care Services (Vård & Omsorg)" }
                                    option { value: "moving", "Logistics & Moving Company (Flyttfirma)" }
                                    option { value: "school", "Educational Institution & School (Skola)" }
                                    option { value: "general", "General Corporate Workspace" }
                                }
                            }

                            div { class: "space-y-2 md:col-span-2",
                                label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Workspace Primary Theme Color (HSL or Hex)" }
                                div { class: "flex items-center gap-3",
                                    input {
                                        class: "w-full px-4 py-2.5 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                                        value: "{brand_color.read()}",
                                        oninput: move |e| brand_color.set(e.value())
                                    }
                                    div {
                                        class: "h-10 w-12 rounded-xl border border-border/40 shadow-inner shrink-0",
                                        style: "background-color: {brand_color.read()};"
                                    }
                                }
                            }
                        }
                    }
                },
                2 => rsx! {
                    div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-6 animate-in fade-in duration-200",
                        div { class: "border-b border-border/30 pb-3",
                            h2 { class: "text-lg font-bold text-foreground", "Step 2: Module Activation" }
                            p { class: "text-xs text-muted-foreground", "Enable or disable operational functional blocks for your workspace." }
                        }

                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            ModuleToggleCard {
                                title: "Instant Messaging",
                                description: "P2P & Encrypted group messaging feed",
                                icon: "message-square",
                                enabled: *module_messaging.read(),
                                on_toggle: move |_| {
                                    let cur = { *module_messaging.read() };
                                    module_messaging.set(!cur);
                                }
                            }
                            ModuleToggleCard {
                                title: "Shift & Event Scheduling",
                                description: "CalDAV-compatible roster calendar & shift booking",
                                icon: "calendar",
                                enabled: *module_scheduling.read(),
                                on_toggle: move |_| {
                                    let cur = { *module_scheduling.read() };
                                    module_scheduling.set(!cur);
                                }
                            }
                            ModuleToggleCard {
                                title: "Daily Care Notes",
                                description: "Zero-Knowledge journal notes with revision audit history",
                                icon: "book-open",
                                enabled: *module_notes.read(),
                                on_toggle: move |_| {
                                    let cur = { *module_notes.read() };
                                    module_notes.set(!cur);
                                }
                            }
                            ModuleToggleCard {
                                title: "Time Tracking & Attest",
                                description: "Employee time punch and monthly manager attestations",
                                icon: "clock",
                                enabled: *module_time.read(),
                                on_toggle: move |_| {
                                    let cur = { *module_time.read() };
                                    module_time.set(!cur);
                                }
                            }
                            ModuleToggleCard {
                                title: "Analytics & Compliance Reports",
                                description: "RUT deduction & operational compliance reports",
                                icon: "bar-chart-3",
                                enabled: *module_reporting.read(),
                                on_toggle: move |_| {
                                    let cur = { *module_reporting.read() };
                                    module_reporting.set(!cur);
                                }
                            }
                            ModuleToggleCard {
                                title: "Care Journaling & Meds",
                                description: "SITHS-card integrated care journals and medication tracking",
                                icon: "heart",
                                enabled: *module_care.read(),
                                on_toggle: move |_| {
                                    let cur = { *module_care.read() };
                                    module_care.set(!cur);
                                }
                            }
                        }
                    }
                },
                3 => rsx! {
                    div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-6 animate-in fade-in duration-200",
                        div { class: "border-b border-border/30 pb-3",
                            h2 { class: "text-lg font-bold text-foreground", "Step 3: Initial Team Invitations" }
                            p { class: "text-xs text-muted-foreground", "Invite your team members via email with role assignments." }
                        }

                        div { class: "flex items-end gap-3",
                            div { class: "flex-1 space-y-1.5",
                                label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Member Email" }
                                input {
                                    class: "w-full px-4 py-2 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                                    placeholder: "colleague@company.com",
                                    value: "{invite_email.read()}",
                                    oninput: move |e| invite_email.set(e.value())
                                }
                            }
                            div { class: "w-44 space-y-1.5",
                                label { class: "text-xs font-semibold text-foreground uppercase tracking-wider", "Role" }
                                select {
                                    class: "w-full px-3 py-2 rounded-xl border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all",
                                    value: "{invite_role.read()}",
                                    onchange: move |e| invite_role.set(e.value()),
                                    option { value: "admin", "Administrator" }
                                    option { value: "manager", "Manager" }
                                    option { value: "member", "Member" }
                                    option { value: "viewer", "Viewer" }
                                }
                            }
                            button {
                                class: "px-4 py-2 rounded-xl bg-primary text-primary-foreground text-sm font-semibold hover:opacity-90 transition-all flex items-center gap-1.5 shadow-sm",
                                onclick: add_invite,
                                components::LucideIcon { name: "plus", class: "h-4 w-4" }
                                "Add Invite"
                            }
                        }

                        if invite_list.read().is_empty() {
                            div { class: "py-8 text-center border border-dashed border-border/60 rounded-xl bg-muted/20 text-muted-foreground text-xs",
                                "No invitations staged. You can also generate invitation links anytime in the Admin Panel."
                            }
                        } else {
                            div { class: "divide-y divide-border/30 border border-border/40 rounded-xl overflow-hidden bg-background",
                                for (idx, (e_mail, r_ole)) in invite_list.read().iter().enumerate() {
                                    div { key: "{idx}", class: "p-3 flex items-center justify-between hover:bg-muted/30 transition-colors",
                                        div { class: "flex items-center gap-3",
                                            components::LucideIcon { name: "mail", class: "h-4 w-4 text-primary" }
                                            div {
                                                p { class: "text-sm font-medium text-foreground m-0", "{e_mail}" }
                                                span { class: "text-xs text-muted-foreground uppercase font-semibold", "{r_ole}" }
                                            }
                                        }
                                        button {
                                            class: "p-1.5 rounded-lg text-red-500 hover:bg-red-500/10 transition-colors",
                                            onclick: move |_| remove_invite(idx),
                                            components::LucideIcon { name: "trash-2", class: "h-4 w-4" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                4 => rsx! {
                    div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-6 animate-in fade-in duration-200",
                        div { class: "border-b border-border/30 pb-3",
                            h2 { class: "text-lg font-bold text-foreground", "Step 4: Security & Compliance Controls" }
                            p { class: "text-xs text-muted-foreground", "Review cryptographic trust keys, database WAL isolation, and audit log tracking." }
                        }

                        div { class: "space-y-4",
                            div { class: "p-4 rounded-xl border border-primary/20 bg-primary/5 flex items-start gap-4",
                                components::LucideIcon { name: "shield-check", class: "h-6 w-6 text-primary shrink-0 mt-0.5" }
                                div { class: "space-y-1",
                                    h4 { class: "text-sm font-bold text-foreground m-0", "Ed25519 Cryptographic Signature Verification" }
                                    p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                                        "All user role modifications and administrative settings changes are signed with your workspace private key to ensure non-repudiation."
                                    }
                                }
                            }

                            div { class: "p-4 rounded-xl border border-border/40 bg-card flex items-start gap-4",
                                components::LucideIcon { name: "database", class: "h-6 w-6 text-primary shrink-0 mt-0.5" }
                                div { class: "space-y-1",
                                    h4 { class: "text-sm font-bold text-foreground m-0", "Tamper-Evident BLAKE3 Hash-Chain Audit Logging" }
                                    p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                                        "Audit logs are continuously hashed into an append-only chain stored in embedded libSQL / OPFS storage, ready for instant export."
                                    }
                                }
                            }
                        }
                    }
                },
                _ => rsx! {
                    div { class: "bg-card border border-border/40 rounded-2xl p-8 text-center space-y-6 shadow-sm animate-in fade-in duration-200",
                        div { class: "h-16 w-16 rounded-full bg-primary/10 border border-primary/30 text-primary flex items-center justify-center mx-auto shadow-inner",
                            components::LucideIcon { name: "rocket", class: "h-8 w-8" }
                        }
                        div { class: "space-y-2 max-w-md mx-auto",
                            h2 { class: "text-2xl font-extrabold text-foreground", "Ready to Launch!" }
                            p { class: "text-sm text-muted-foreground",
                                "Click below to finalize workspace onboarding, apply active modules, and start collaborating with your team."
                            }
                        }

                        div { class: "pt-4",
                            button {
                                class: "px-8 py-3 rounded-xl bg-primary text-primary-foreground font-bold text-base hover:scale-102 hover:shadow-lg active:scale-98 transition-all flex items-center gap-2 mx-auto disabled:opacity-50",
                                disabled: *is_saving.read(),
                                onclick: handle_finish,
                                if *is_saving.read() {
                                    components::LucideIcon { name: "loader-2", class: "h-5 w-5 animate-spin" }
                                    span { "Finalizing Setup..." }
                                } else {
                                    components::LucideIcon { name: "check-circle", class: "h-5 w-5" }
                                    span { "Complete & Launch Workspace" }
                                }
                            }
                        }
                    }
                }
            }

            // Wizard Step Navigation Footer
            div { class: "flex items-center justify-between border-t border-border/40 pt-4",
                button {
                    class: "px-5 py-2 rounded-xl border border-input bg-card text-foreground font-medium text-sm hover:bg-accent transition-colors flex items-center gap-2 disabled:opacity-40",
                    disabled: *step.read() <= 1,
                    onclick: move |_| {
                        let cur = *step.read();
                        if cur > 1 { step.set(cur - 1); }
                    },
                    components::LucideIcon { name: "chevron-left", class: "h-4 w-4" }
                    "Previous"
                }

                if *step.read() < 5 {
                    button {
                        class: "px-5 py-2 rounded-xl bg-primary text-primary-foreground font-semibold text-sm hover:opacity-90 transition-all flex items-center gap-2 shadow-sm",
                        onclick: move |_| {
                            let cur = *step.read();
                            if cur < 5 { step.set(cur + 1); }
                        },
                        "Next Step"
                        components::LucideIcon { name: "chevron-right", class: "h-4 w-4" }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ModuleToggleProps {
    title: &'static str,
    description: &'static str,
    icon: &'static str,
    enabled: bool,
    on_toggle: EventHandler<()>,
}

#[component]
fn ModuleToggleCard(props: ModuleToggleProps) -> Element {
    let enabled = props.enabled;
    let on_toggle = props.on_toggle;
    rsx! {
        div {
            class: format!(
                "p-4 rounded-xl border transition-all cursor-pointer flex items-start gap-4 select-none {}",
                if enabled {
                    "border-primary/50 bg-primary/5 shadow-sm"
                } else {
                    "border-border/40 bg-muted/20 opacity-70 hover:opacity-100"
                }
            ),
            onclick: move |_| on_toggle.call(()),
            div { class: "h-10 w-10 rounded-lg bg-primary/10 text-primary flex items-center justify-center shrink-0 mt-0.5",
                components::LucideIcon { name: props.icon, class: "h-5 w-5" }
            }
            div { class: "flex-1 space-y-1",
                div { class: "flex items-center justify-between",
                    h4 { class: "text-sm font-bold text-foreground m-0", "{props.title}" }
                    div { class: format!("w-3 h-3 rounded-full {}", if enabled { "bg-emerald-500 shadow-sm" } else { "bg-zinc-400/50" }) }
                }
                p { class: "text-xs text-muted-foreground m-0 leading-relaxed", "{props.description}" }
            }
        }
    }
}
