use crate::components;
use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct BillingEngineProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn BillingEngineView(props: BillingEngineProps) -> Element {
    let mut selected_tier = use_signal(|| "starter".to_string());
    let mut selected_platform = use_signal(|| "stripe".to_string());
    let mut allocated_seats = use_signal(|| 5u32);
    let mut billing_cycle = use_signal(|| "monthly".to_string());

    let mut is_updating = use_signal(|| false);
    let mut is_generating_inv = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let active_user_id = props.active_user.id.clone();
    let workspace_id = props.workspace.id.clone();
    let mut db_trigger = props.db_trigger;

    // Asynchronous resource for current subscription
    let subscription_res = use_resource(move || {
        let uid = active_user_id.clone();
        let wsid = workspace_id.clone();
        let _trig = *db_trigger.read();
        async move { yntra_core::get_workspace_subscription(uid, wsid).await }
    });

    let active_user_id_inv = props.active_user.id.clone();
    let workspace_id_inv = props.workspace.id.clone();

    // Asynchronous resource for invoices list
    let invoices_res = use_resource(move || {
        let uid = active_user_id_inv.clone();
        let wsid = workspace_id_inv.clone();
        let _trig = *db_trigger.read();
        async move { yntra_core::get_platform_invoices(uid, wsid).await }
    });

    // Populate local signal defaults from fetched subscription
    use_effect(move || {
        if let Some(Ok(sub)) = &*subscription_res.read() {
            selected_tier.set(sub.tier.clone());
            selected_platform.set(sub.payment_platform.clone());
            allocated_seats.set(sub.seats_allocated);
            billing_cycle.set(sub.billing_cycle.clone());
        }
    });

    let sub_val = match &*subscription_res.read() {
        Some(Ok(s)) => Some(s.clone()),
        _ => None,
    };

    let current_tier = sub_val
        .as_ref()
        .map(|s| s.tier.as_str())
        .unwrap_or("starter");
    let current_platform = sub_val
        .as_ref()
        .map(|s| s.payment_platform.as_str())
        .unwrap_or("stripe");
    let price_per_seat = yntra_core::get_tier_seat_price(&selected_tier.read());
    let seats_count = *allocated_seats.read();
    let annual_discount = if *billing_cycle.read() == "annual" {
        0.85
    } else {
        1.0
    };
    let monthly_total = price_per_seat * (seats_count as f64) * annual_discount;

    let handle_save_subscription = {
        let req_uid = props.active_user.id.clone();
        let wsid = props.workspace.id.clone();
        move |_| {
            is_updating.set(true);
            error_msg.set(None);
            status_msg.set(None);

            let uid = req_uid.clone();
            let ws = wsid.clone();
            let tier = selected_tier.read().clone();
            let platform = selected_platform.read().clone();
            let seats = *allocated_seats.read();
            let cycle = billing_cycle.read().clone();

            spawn(async move {
                match yntra_core::update_workspace_subscription(
                    uid,
                    ws,
                    tier.clone(),
                    platform.clone(),
                    seats,
                    cycle,
                )
                .await
                {
                    Ok(s) => {
                        is_updating.set(false);
                        status_msg.set(Some(format!(
                            "Subscription updated successfully! Active tier: {}, Platform: {}, Seats: {}",
                            s.tier.to_uppercase(), s.payment_platform.to_uppercase(), s.seats_allocated
                        )));
                        let trig_val = *db_trigger.read();
                        db_trigger.set(trig_val + 1);
                    }
                    Err(e) => {
                        is_updating.set(false);
                        error_msg.set(Some(format!("Failed to update subscription: {}", e)));
                    }
                }
            });
        }
    };

    let handle_checkout_session = {
        let req_uid = props.active_user.id.clone();
        let wsid = props.workspace.id.clone();
        move |_| {
            is_updating.set(true);
            error_msg.set(None);
            status_msg.set(None);

            let uid = req_uid.clone();
            let ws = wsid.clone();
            let platform = selected_platform.read().clone();
            let tier = selected_tier.read().clone();
            let seats = *allocated_seats.read();

            spawn(async move {
                match yntra_core::create_payment_checkout_session(uid, ws, platform, tier, seats)
                    .await
                {
                    Ok(sess) => {
                        is_updating.set(false);
                        status_msg.set(Some(format!(
                            "Checkout session created on {} ({})! Total: ${:.2}. Launch URL: {}",
                            sess.payment_platform.to_uppercase(),
                            sess.session_id,
                            sess.total_amount,
                            sess.checkout_url
                        )));
                    }
                    Err(e) => {
                        is_updating.set(false);
                        error_msg.set(Some(format!("Failed to create checkout session: {}", e)));
                    }
                }
            });
        }
    };

    let handle_generate_invoice = {
        let req_uid = props.active_user.id.clone();
        let wsid = props.workspace.id.clone();
        move |_| {
            is_generating_inv.set(true);
            error_msg.set(None);
            status_msg.set(None);

            let uid = req_uid.clone();
            let ws = wsid.clone();

            spawn(async move {
                match yntra_core::generate_automated_invoice(uid, ws).await {
                    Ok(inv) => {
                        is_generating_inv.set(false);
                        status_msg.set(Some(format!(
                            "Invoice {} successfully generated for ${:.2} ({})",
                            inv.invoice_number,
                            inv.amount_paid,
                            inv.payment_platform.to_uppercase()
                        )));
                        let trig_val = *db_trigger.read();
                        db_trigger.set(trig_val + 1);
                    }
                    Err(e) => {
                        is_generating_inv.set(false);
                        error_msg.set(Some(format!("Failed to generate invoice: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "p-6 space-y-8 animate-in fade-in duration-300",
            // Header Card
            div { class: "flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-border/40 pb-4",
                div { class: "space-y-1",
                    h2 { class: "text-2xl font-extrabold text-foreground tracking-tight flex items-center gap-2",
                        components::LucideIcon { name: "credit-card", class: "h-6 w-6 text-primary" }
                        "Billing & Subscription Engine"
                    }
                    p { class: "text-xs text-muted-foreground",
                        "Manage seat-based pricing, payment platform integrations (Stripe, RevenueCat, Chargebee), tier features & automated invoicing."
                    }
                }

                div { class: "flex items-center gap-3",
                    span { class: "px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider bg-primary/10 text-primary border border-primary/20",
                        "Active Plan: {current_tier}"
                    }
                    span { class: "px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider bg-emerald-500/10 text-emerald-500 border border-emerald-500/20",
                        "Provider: {current_platform}"
                    }
                }
            }

            // Alerts
            if let Some(err) = error_msg.read().as_ref() {
                div { class: "p-3.5 rounded-xl bg-red-500/10 border border-red-500/30 text-red-500 text-xs font-medium flex items-center gap-2.5",
                    components::LucideIcon { name: "alert-circle", class: "h-4 w-4 shrink-0" }
                    span { "{err}" }
                }
            }
            if let Some(msg) = status_msg.read().as_ref() {
                div { class: "p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-500 text-xs font-medium flex items-center gap-2.5",
                    components::LucideIcon { name: "check-circle-2", class: "h-4 w-4 shrink-0" }
                    span { "{msg}" }
                }
            }

            // 1. Payment Platform Integrations Selector
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-4",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "sliders", class: "h-4 w-4 text-primary" }
                        "Select Payment Platform Gateway"
                    }
                    span { class: "text-xs text-muted-foreground font-medium", "Active multi-provider integration engine" }
                }

                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                    div {
                        class: format!(
                            "p-4 rounded-xl border cursor-pointer transition-all flex flex-col justify-between space-y-3 {}",
                            if *selected_platform.read() == "stripe" {
                                "border-primary bg-primary/5 ring-2 ring-primary/20"
                            } else {
                                "border-border/40 bg-background hover:bg-accent/40"
                            }
                        ),
                        onclick: move |_| selected_platform.set("stripe".to_string()),
                        div { class: "flex items-center justify-between",
                            span { class: "font-extrabold text-sm text-foreground", "Stripe Connect" }
                            components::LucideIcon { name: "credit-card", class: "h-5 w-5 text-indigo-500" }
                        }
                        p { class: "text-xs text-muted-foreground m-0", "Global card processing, ACH debit & direct web checkout sessions." }
                        span { class: "text-[10px] font-bold text-indigo-400 uppercase tracking-wider", "Web & Server API" }
                    }

                    div {
                        class: format!(
                            "p-4 rounded-xl border cursor-pointer transition-all flex flex-col justify-between space-y-3 {}",
                            if *selected_platform.read() == "revenuecat" {
                                "border-primary bg-primary/5 ring-2 ring-primary/20"
                            } else {
                                "border-border/40 bg-background hover:bg-accent/40"
                            }
                        ),
                        onclick: move |_| selected_platform.set("revenuecat".to_string()),
                        div { class: "flex items-center justify-between",
                            span { class: "font-extrabold text-sm text-foreground", "RevenueCat" }
                            components::LucideIcon { name: "smartphone", class: "h-5 w-5 text-rose-500" }
                        }
                        p { class: "text-xs text-muted-foreground m-0", "Cross-platform mobile In-App purchases, iOS App Store & Google Play." }
                        span { class: "text-[10px] font-bold text-rose-400 uppercase tracking-wider", "Mobile & App Entitlements" }
                    }

                    div {
                        class: format!(
                            "p-4 rounded-xl border cursor-pointer transition-all flex flex-col justify-between space-y-3 {}",
                            if *selected_platform.read() == "chargebee" {
                                "border-primary bg-primary/5 ring-2 ring-primary/20"
                            } else {
                                "border-border/40 bg-background hover:bg-accent/40"
                            }
                        ),
                        onclick: move |_| selected_platform.set("chargebee".to_string()),
                        div { class: "flex items-center justify-between",
                            span { class: "font-extrabold text-sm text-foreground", "Chargebee" }
                            components::LucideIcon { name: "building-2", class: "h-5 w-5 text-amber-500" }
                        }
                        p { class: "text-xs text-muted-foreground m-0", "Enterprise B2B billing portals, custom contracts & recurring invoices." }
                        span { class: "text-[10px] font-bold text-amber-400 uppercase tracking-wider", "B2B Subscription Management" }
                    }
                }
            }

            // 2. Subscription Feature Tiers & Pricing Cards
            div { class: "space-y-4",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "layers", class: "h-4 w-4 text-primary" }
                        "Feature Tiers & Seat Pricing"
                    }

                    div { class: "inline-flex items-center gap-2 p-1 rounded-xl bg-muted/40 border border-border/40 text-xs font-medium",
                        button {
                            class: format!(
                                "px-3 py-1 rounded-lg transition-all {}",
                                if *billing_cycle.read() == "monthly" { "bg-background text-foreground shadow-sm font-semibold" } else { "text-muted-foreground" }
                            ),
                            onclick: move |_| billing_cycle.set("monthly".to_string()),
                            "Monthly"
                        }
                        button {
                            class: format!(
                                "px-3 py-1 rounded-lg transition-all {}",
                                if *billing_cycle.read() == "annual" { "bg-background text-foreground shadow-sm font-semibold" } else { "text-muted-foreground" }
                            ),
                            onclick: move |_| billing_cycle.set("annual".to_string()),
                            "Annual (Save 15%)"
                        }
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                    // Starter Tier
                    div {
                        class: format!(
                            "bg-card border rounded-2xl p-6 shadow-sm flex flex-col justify-between space-y-6 transition-all {}",
                            if *selected_tier.read() == "starter" { "border-primary ring-2 ring-primary/20" } else { "border-border/40" }
                        ),
                        div { class: "space-y-3",
                            div { class: "flex items-center justify-between",
                                h4 { class: "text-lg font-extrabold text-foreground m-0", "Starter" }
                                if current_tier == "starter" {
                                    span { class: "px-2 py-0.5 rounded text-[10px] font-extrabold bg-primary/10 text-primary border border-primary/20 uppercase", "Current" }
                                }
                            }
                            p { class: "text-xs text-muted-foreground m-0", "Essential operational tools for small team setups." }
                            div { class: "flex items-baseline gap-1",
                                span { class: "text-3xl font-black text-foreground", "$15" }
                                span { class: "text-xs text-muted-foreground", "/ seat / mo" }
                            }
                            ul { class: "space-y-2 text-xs text-muted-foreground pt-3 border-t border-border/30 list-none p-0 m-0",
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "P2P Encrypted Messaging" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Task & Todo Boards" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Basic Time Punch Reports" }
                            }
                        }
                        button {
                            class: "w-full py-2.5 rounded-xl border border-input bg-background hover:bg-accent text-xs font-bold text-foreground transition-all",
                            onclick: move |_| selected_tier.set("starter".to_string()),
                            "Select Starter Tier"
                        }
                    }

                    // Pro Tier
                    div {
                        class: format!(
                            "bg-card border rounded-2xl p-6 shadow-sm flex flex-col justify-between space-y-6 transition-all relative overflow-hidden {}",
                            if *selected_tier.read() == "pro" { "border-primary ring-2 ring-primary/20 bg-primary/5" } else { "border-border/40" }
                        ),
                        div { class: "space-y-3",
                            div { class: "flex items-center justify-between",
                                h4 { class: "text-lg font-extrabold text-foreground m-0", "Pro" }
                                span { class: "px-2 py-0.5 rounded text-[10px] font-extrabold bg-amber-500/10 text-amber-500 border border-amber-500/20 uppercase", "Popular" }
                            }
                            p { class: "text-xs text-muted-foreground m-0", "Advanced dispatch, CalDAV & automated RUT exports." }
                            div { class: "flex items-baseline gap-1",
                                span { class: "text-3xl font-black text-foreground", "$35" }
                                span { class: "text-xs text-muted-foreground", "/ seat / mo" }
                            }
                            ul { class: "space-y-2 text-xs text-muted-foreground pt-3 border-t border-border/30 list-none p-0 m-0",
                                li { class: "flex items-center gap-2 font-medium text-foreground", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Everything in Starter" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "CalDAV Roster Sync" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Automated RUT Tax Exports" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Advanced Analytics & Reports" }
                            }
                        }
                        button {
                            class: "w-full py-2.5 rounded-xl bg-primary text-primary-foreground font-bold text-xs hover:opacity-90 transition-all shadow-sm",
                            onclick: move |_| selected_tier.set("pro".to_string()),
                            "Select Pro Tier"
                        }
                    }

                    // Enterprise Tier
                    div {
                        class: format!(
                            "bg-card border rounded-2xl p-6 shadow-sm flex flex-col justify-between space-y-6 transition-all {}",
                            if *selected_tier.read() == "enterprise" { "border-primary ring-2 ring-primary/20" } else { "border-border/40" }
                        ),
                        div { class: "space-y-3",
                            div { class: "flex items-center justify-between",
                                h4 { class: "text-lg font-extrabold text-foreground m-0", "Enterprise" }
                                if current_tier == "enterprise" {
                                    span { class: "px-2 py-0.5 rounded text-[10px] font-extrabold bg-primary/10 text-primary border border-primary/20 uppercase", "Current" }
                                }
                            }
                            p { class: "text-xs text-muted-foreground m-0", "SITHS Card Auth, hardware keys & compliance audit history." }
                            div { class: "flex items-baseline gap-1",
                                span { class: "text-3xl font-black text-foreground", "$75" }
                                span { class: "text-xs text-muted-foreground", "/ seat / mo" }
                            }
                            ul { class: "space-y-2 text-xs text-muted-foreground pt-3 border-t border-border/30 list-none p-0 m-0",
                                li { class: "flex items-center gap-2 font-medium text-foreground", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Everything in Pro" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "SITHS Card & NFC Auth" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Custom Branding & Domain" }
                                li { class: "flex items-center gap-2", components::LucideIcon { name: "check", class: "h-3.5 w-3.5 text-emerald-500" }, "Unlimited Ed25519 Audit Chain" }
                            }
                        }
                        button {
                            class: "w-full py-2.5 rounded-xl border border-input bg-background hover:bg-accent text-xs font-bold text-foreground transition-all",
                            onclick: move |_| selected_tier.set("enterprise".to_string()),
                            "Select Enterprise Tier"
                        }
                    }
                }
            }

            // 3. Seat Allocation & Summary
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-6",
                h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                    components::LucideIcon { name: "users", class: "h-4 w-4 text-primary" }
                    "Seat License Allocation & Price Calculation"
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 gap-6 items-center",
                    div { class: "space-y-3",
                        label { class: "text-xs font-semibold text-foreground flex justify-between",
                            span { "Allocated Seats Count" }
                            span { class: "font-mono font-bold text-primary", "{seats_count} Seats" }
                        }
                        div { class: "flex items-center gap-3",
                            button {
                                class: "h-10 w-10 rounded-xl border border-input bg-background hover:bg-accent font-bold text-lg text-foreground flex items-center justify-center transition-colors",
                                onclick: move |_| {
                                    let cur = *allocated_seats.read();
                                    if cur > 1 { allocated_seats.set(cur - 1); }
                                },
                                "-"
                            }
                            input {
                                type: "range",
                                min: "1",
                                max: "100",
                                value: "{seats_count}",
                                class: "w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer",
                                oninput: move |e: Event<FormData>| {
                                    if let Ok(v) = e.value().parse::<u32>() {
                                        allocated_seats.set(v.max(1));
                                    }
                                }
                            }
                            button {
                                class: "h-10 w-10 rounded-xl border border-input bg-background hover:bg-accent font-bold text-lg text-foreground flex items-center justify-center transition-colors",
                                onclick: move |_| {
                                    let cur = *allocated_seats.read();
                                    allocated_seats.set(cur + 1);
                                },
                                "+"
                            }
                        }
                    }

                    div { class: "p-4 rounded-xl bg-muted/20 border border-border/30 space-y-2",
                        div { class: "flex justify-between text-xs text-muted-foreground",
                            span { "Tier Rate:" }
                            span { class: "font-mono font-semibold text-foreground", "${price_per_seat} / seat" }
                        }
                        div { class: "flex justify-between text-xs text-muted-foreground",
                            span { "Discount:" }
                            span { class: "font-mono font-semibold text-emerald-500", { if *billing_cycle.read() == "annual" { "15% Annual Discount" } else { "None (Monthly)" } } }
                        }
                        div { class: "flex justify-between text-sm font-extrabold text-foreground pt-2 border-t border-border/30",
                            span { "Estimated Monthly Total:" }
                            span { class: "font-mono text-primary text-base", "${monthly_total:.2} USD" }
                        }
                    }
                }

                div { class: "flex flex-wrap items-center justify-end gap-3 pt-4 border-t border-border/30",
                    button {
                        class: "px-4 py-2 rounded-xl border border-input bg-background hover:bg-accent text-xs font-semibold text-foreground transition-all inline-flex items-center gap-2",
                        disabled: *is_updating.read(),
                        onclick: handle_checkout_session,
                        components::LucideIcon { name: "external-link", class: "h-3.5 w-3.5" }
                        "Launch Gateway Checkout"
                    }
                    button {
                        class: "px-5 py-2.5 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-90 transition-all shadow-sm inline-flex items-center gap-2",
                        disabled: *is_updating.read(),
                        onclick: handle_save_subscription,
                        components::LucideIcon { name: "save", class: "h-4 w-4" }
                        {if *is_updating.read() { "Updating..." } else { "Save Subscription" }}
                    }
                }
            }

            // 4. Feature Gate Inspector Matrix
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-4",
                div { class: "flex items-center justify-between",
                    h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "shield-check", class: "h-4 w-4 text-primary" }
                        "Feature Tier Gate Inspector"
                    }
                    span { class: "text-xs text-muted-foreground", "Live entitlement verification engine" }
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 text-xs",
                    div { class: "p-3.5 rounded-xl border border-border/30 bg-background space-y-1.5",
                        div { class: "flex items-center justify-between",
                            span { class: "font-bold text-foreground", "CalDAV Sync" }
                            span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-amber-500/10 text-amber-500 border border-amber-500/20", "Pro" }
                        }
                        p { class: "text-muted-foreground m-0 text-[11px]", "Roster calendar sync & external shift booking." }
                    }

                    div { class: "p-3.5 rounded-xl border border-border/30 bg-background space-y-1.5",
                        div { class: "flex items-center justify-between",
                            span { class: "font-bold text-foreground", "RUT Tax Exports" }
                            span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-amber-500/10 text-amber-500 border border-amber-500/20", "Pro" }
                        }
                        p { class: "text-muted-foreground m-0 text-[11px]", "Skatteverket automated RUT claim manifest." }
                    }

                    div { class: "p-3.5 rounded-xl border border-border/30 bg-background space-y-1.5",
                        div { class: "flex items-center justify-between",
                            span { class: "font-bold text-foreground", "SITHS Card Auth" }
                            span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-indigo-500/10 text-indigo-500 border border-indigo-500/20", "Enterprise" }
                        }
                        p { class: "text-muted-foreground m-0 text-[11px]", "Smart card hardware identity & care journal sign-off." }
                    }

                    div { class: "p-3.5 rounded-xl border border-border/30 bg-background space-y-1.5",
                        div { class: "flex items-center justify-between",
                            span { class: "font-bold text-foreground", "Custom Branding" }
                            span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-indigo-500/10 text-indigo-500 border border-indigo-500/20", "Enterprise" }
                        }
                        p { class: "text-muted-foreground m-0 text-[11px]", "White-label domain, custom logo & email templates." }
                    }
                }
            }

            // 5. Automated Platform Invoices Table
            div { class: "bg-card border border-border/40 rounded-2xl p-6 shadow-sm space-y-4",
                div { class: "flex items-center justify-between border-b border-border/30 pb-3",
                    div { class: "space-y-0.5",
                        h3 { class: "text-base font-bold text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "receipt", class: "h-4 w-4 text-primary" }
                            "Automated Platform Invoices"
                        }
                        p { class: "text-xs text-muted-foreground m-0", "Automated seat billing invoices and PDF/JSON data exports." }
                    }

                    button {
                        class: "px-3.5 py-1.5 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 transition-all shadow-sm inline-flex items-center gap-1.5",
                        disabled: *is_generating_inv.read(),
                        onclick: handle_generate_invoice,
                        components::LucideIcon { name: "file-plus", class: "h-3.5 w-3.5" }
                        {if *is_generating_inv.read() { "Generating..." } else { "Generate Invoice" }}
                    }
                }

                match &*invoices_res.read() {
                    Some(Ok(invoices)) if !invoices.is_empty() => rsx! {
                        div { class: "overflow-x-auto rounded-xl border border-border/30 bg-background",
                            table { class: "w-full text-left border-collapse",
                                thead {
                                    tr { class: "border-b border-border/40 bg-muted/40 text-xs font-semibold text-muted-foreground uppercase tracking-wider",
                                        th { class: "p-3.5", "Invoice #" }
                                        th { class: "p-3.5", "Gateway" }
                                        th { class: "p-3.5", "Seats" }
                                        th { class: "p-3.5", "Amount Paid" }
                                        th { class: "p-3.5", "Status" }
                                        th { class: "p-3.5 text-right", "Actions" }
                                    }
                                }
                                tbody { class: "divide-y divide-border/20 text-xs text-foreground font-mono",
                                    for inv in invoices.iter() {
                                        {
                                            let inv_id = inv.id.clone();
                                            let req_uid = props.active_user.id.clone();
                                            let wsid = props.workspace.id.clone();
                                            rsx! {
                                                tr { key: "{inv.id}", class: "hover:bg-muted/20 transition-colors",
                                                    td { class: "p-3.5 font-bold text-primary", "{inv.invoice_number}" }
                                                    td { class: "p-3.5 font-sans uppercase font-medium text-foreground", "{inv.payment_platform}" }
                                                    td { class: "p-3.5 text-muted-foreground", "{inv.seat_count} seats" }
                                                    td { class: "p-3.5 font-bold text-foreground", "${inv.amount_paid:.2} {inv.currency}" }
                                                    td { class: "p-3.5",
                                                        span { class: "px-2.5 py-0.5 rounded-full text-xs font-bold bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 uppercase",
                                                            "{inv.status}"
                                                        }
                                                    }
                                                    td { class: "p-3.5 text-right font-sans space-x-2",
                                                        button {
                                                            class: "px-2.5 py-1 rounded-lg border border-input bg-background hover:bg-accent text-xs font-medium text-foreground transition-all inline-flex items-center gap-1",
                                                            onclick: {
                                                                let uid = req_uid.clone();
                                                                let ws = wsid.clone();
                                                                let iid = inv_id.clone();
                                                                move |_| {
                                                                    let u = uid.clone();
                                                                    let w = ws.clone();
                                                                    let id = iid.clone();
                                                                    spawn(async move {
                                                                        if let Ok(pdf) = yntra_core::export_platform_invoice_pdf(u, w, id).await {
                                                                            status_msg.set(Some(format!("Downloaded PDF Document ({} bytes)", pdf.len())));
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            components::LucideIcon { name: "download", class: "h-3 w-3" }
                                                            "PDF"
                                                        }
                                                        button {
                                                            class: "px-2.5 py-1 rounded-lg border border-input bg-background hover:bg-accent text-xs font-medium text-foreground transition-all inline-flex items-center gap-1",
                                                            onclick: {
                                                                let uid = req_uid.clone();
                                                                let ws = wsid.clone();
                                                                let iid = inv_id.clone();
                                                                move |_| {
                                                                    let u = uid.clone();
                                                                    let w = ws.clone();
                                                                    let id = iid.clone();
                                                                    spawn(async move {
                                                                        if let Ok(json) = yntra_core::export_platform_invoice_json(u, w, id).await {
                                                                            status_msg.set(Some(format!("Exported Invoice JSON: {}", &json[..json.len().min(60)])));
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            components::LucideIcon { name: "code", class: "h-3 w-3" }
                                                            "JSON"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        div { class: "py-8 text-center border border-dashed border-border/60 rounded-xl bg-muted/20 text-muted-foreground space-y-1.5",
                            components::LucideIcon { name: "receipt", class: "h-6 w-6 mx-auto text-muted-foreground/60" }
                            p { class: "text-xs font-medium m-0", "No automated platform invoices generated yet" }
                        }
                    },
                    _ => rsx! {
                        div { class: "p-4 text-center text-xs text-muted-foreground", "Loading invoices history..." }
                    }
                }
            }
        }
    }
}
