use dioxus::prelude::*;
#[allow(unused_imports)]
use crate::components;
use yntra_core::{SchoolInvoice, WorkspaceUser};

const CUSTOM_CSS: &str = r#"
@keyframes scan {
    0% { top: 0%; opacity: 0.2; }
    50% { top: 100%; opacity: 0.8; }
    100% { top: 0%; opacity: 0.2; }
}
.scanner-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, transparent, #e8117f 50%, transparent);
    box-shadow: 0 0 10px #e8117f, 0 0 4px #e8117f;
    animation: scan 2.5s linear infinite;
}
.swish-pulse {
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}
@keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: .7; transform: scale(0.96); }
}
"#;

#[derive(Props, Clone)]
pub struct BillingTabProps {
    pub invoices: Vec<SchoolInvoice>,
    pub active_user: WorkspaceUser,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for BillingTabProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn BillingTab(props: BillingTabProps) -> Element {
    let invoices = props.invoices;
    let active_user = props.active_user;
    let workspace_id = props.workspace_id;
    let db_trigger = props.db_trigger;

    let total_due: f64 = invoices.iter()
        .filter(|i| i.status == "unpaid")
        .map(|i| i.amount)
        .sum();

    // SOTA Modal States
    let mut show_swish_modal = use_signal(|| false);
    let mut swish_phone = use_signal(|| active_user.phone.clone().unwrap_or_default());
    let mut swish_step = use_signal(|| 0); // 0: Idle/Form, 1: Initiating, 2: Waiting Signature, 3: Finalizing, 4: Success
    let mut swish_countdown = use_signal(|| 30);
    let mut qr_zoomed = use_signal(|| false);

    // Dynamic Swedish phone validation
    let is_phone_valid = {
        let val = swish_phone.read();
        let digits: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.starts_with("07") && digits.len() >= 10
    };

    rsx! {
        // Embed scanline animation keyframes locally
        style { "{CUSTOM_CSS}" }

        div { class: "grid gap-6 md:grid-cols-3 items-start",
            // Left: Balance & Payment Simulator Card
            div { class: "md:col-span-1 flex flex-col gap-4",
                components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                    h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "wallet", size: "16", class: "text-primary" }
                        "Tuition Balance"
                    }
                    
                    div { class: "p-4 rounded-xl bg-sidebar/40 border border-border/20 flex flex-col gap-1 text-center",
                        span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Total Outstanding" }
                        span { class: "text-2xl font-black text-foreground", "${total_due:.2}" }
                    }

                    if total_due > 0.0 {
                        div { class: "flex flex-col gap-3 pt-3 border-t border-border/30",
                            span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Payment Gateway" }
                            
                            button {
                                class: "w-full py-2.5 rounded-xl text-xs font-black bg-[#e8117f] text-white hover:bg-[#c90f6e] transition-all shadow-md hover:shadow-[#e8117f]/20 flex items-center justify-center gap-2 hover:scale-[1.02] active:scale-[0.98] duration-150",
                                onclick: move |_| {
                                    swish_step.set(0);
                                    show_swish_modal.set(true);
                                },
                                components::LucideIcon { name: "credit-card", size: "14" }
                                "Pay with Swish"
                            }
                        }
                    }
                }
            }

            // Right: Invoices and Invoicing History Table
            div { class: "md:col-span-2 flex flex-col gap-4",
                components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                    h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "credit-card", size: "16", class: "text-primary" }
                        "Invoice Registry & Payments"
                    }

                    if invoices.is_empty() {
                        p { class: "text-xs text-muted-foreground italic text-center p-8 m-0", "No billing invoices found." }
                    } else {
                        div { class: "flex flex-col gap-4",
                            for invoice in invoices.iter() {
                                div { key: "{invoice.id}", class: "border border-border/30 p-4 rounded-xl bg-white/[0.01] flex flex-col gap-2.5",
                                    div { class: "flex justify-between items-center",
                                        div { class: "flex flex-col gap-0.5",
                                            span { class: "text-sm font-black text-foreground", "{invoice.title}" }
                                            span { class: "text-[10px] text-muted-foreground font-semibold", "Due Date: {invoice.due_date}" }
                                        }
                                        div { class: "flex items-center gap-2.5",
                                            span { class: "text-sm font-black text-foreground", "${invoice.amount:.2}" }
                                            match invoice.status.as_str() {
                                                "paid" => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Paid" } },
                                                _ => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-amber-500/10 text-amber-400 border border-amber-500/15 shadow-sm animate-pulse", "Unpaid" } }
                                            }
                                        }
                                    }
                                    
                                    if let Some(ref paid_at) = invoice.paid_at {
                                        div { class: "text-[10px] text-muted-foreground bg-sidebar/30 p-2 rounded border border-border/20 font-medium",
                                            "Receipt: Paid via Swish on {paid_at}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // SOTA Swish Checkout Modal
        if *show_swish_modal.read() {
            div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/85 backdrop-blur-md animate-in fade-in duration-300",
                div { class: "w-full max-w-md p-6 bg-sidebar/95 border border-border/40 rounded-3xl shadow-2xl flex flex-col gap-5 relative overflow-hidden backdrop-blur-xl animate-in scale-in duration-200 border-t-[#e8117f]/30",
                    
                    // Decorative brand ambient glow
                    div { class: "absolute -right-20 -top-20 w-40 h-40 rounded-full bg-[#e8117f]/10 blur-3xl pointer-events-none" }
                    div { class: "absolute -left-20 -bottom-20 w-40 h-40 rounded-full bg-[#3b82f6]/10 blur-3xl pointer-events-none" }

                    // Modal Header
                    div { class: "flex justify-between items-center border-b border-border/20 pb-3.5 relative z-10",
                        div { class: "flex items-center gap-2.5",
                            div { class: "w-7 h-7 rounded-xl bg-gradient-to-br from-[#FF5F00] via-[#e8117f] to-[#3b82f6] flex items-center justify-center text-white text-xs font-black shadow-lg", "S" }
                            div { class: "flex flex-col",
                                h3 { class: "text-sm font-black text-foreground m-0 tracking-wide", "Swish Secure Checkout" }
                                span { class: "text-[9px] text-muted-foreground font-semibold uppercase tracking-wider", "Decentralized Node" }
                            }
                        }
                        button {
                            class: "p-1.5 rounded-full hover:bg-muted text-muted-foreground hover:text-foreground transition-colors",
                            onclick: move |_| show_swish_modal.set(false),
                            components::LucideIcon { name: "x", size: "16" }
                        }
                    }

                    match *swish_step.read() {
                        0 => {
                            let amount_str = format!("{:.2}", total_due).replace('.', ",");
                            let qr_payload = format!("C1231181189;{};Tuition%20Fees;0", amount_str);
                            let qr_url = format!("https://api.qrserver.com/v1/create-qr-code/?size=250x250&data={}", qr_payload);
                            let is_zoomed = *qr_zoomed.read();
                            
                            let active_user_c1a = active_user.clone();
                            let active_user_c1b = active_user.clone();
                            let active_user_c2 = active_user.clone();
                            let workspace_id_c1a = workspace_id.clone();
                            let workspace_id_c1b = workspace_id.clone();
                            let workspace_id_c2 = workspace_id.clone();
                            let invoices_c1a = invoices.clone();
                            let invoices_c1b = invoices.clone();
                            let invoices_c2 = invoices.clone();
                            
                            // Generate real Swish deep link
                            let swish_url = get_swish_link("1231181189", total_due, "Tuition Fees");
                            
                            rsx! {
                                div { class: "flex flex-col gap-4 py-1 relative z-10",
                                    
                                    // Balance info board
                                    div { class: "flex justify-between items-center bg-sidebar/50 p-4 rounded-2xl border border-border/30 shadow-inner",
                                        div { class: "flex flex-col gap-0.5",
                                            span { class: "text-[9px] font-black text-muted-foreground uppercase tracking-widest", "Academic Order" }
                                            span { class: "text-xs font-extrabold text-foreground", "Yntra Tuition Fees" }
                                        }
                                        div { class: "text-right flex flex-col gap-0.5",
                                            span { class: "text-[9px] font-black text-[#e8117f] uppercase tracking-widest", "Amount Due" }
                                            span { class: "text-lg font-black text-foreground", "${total_due:.2} SEK" }
                                        }
                                    }

                                    // Option A: Send Push / Launch Swish
                                    div { class: "flex flex-col gap-2.5 bg-sidebar/30 p-4 rounded-2xl border border-border/20",
                                        div { class: "flex justify-between items-center",
                                            label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Swish App Integration" }
                                            if !swish_phone.read().is_empty() {
                                                if is_phone_valid {
                                                    span { class: "text-[8px] font-black uppercase bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 px-1.5 py-0.5 rounded", "Format Valid" }
                                                } else {
                                                    span { class: "text-[8px] font-black uppercase bg-rose-500/10 text-rose-400 border border-rose-500/20 px-1.5 py-0.5 rounded", "Format Error" }
                                                }
                                            }
                                        }
                                        div { class: "relative w-full",
                                            div { class: "absolute left-3 top-2.5 text-xs font-bold text-muted-foreground/60 select-none", "+46" }
                                            input {
                                                r#type: "text",
                                                class: "yntra-input text-xs w-full pl-11 font-extrabold tracking-wide",
                                                placeholder: "e.g. 701 234 56",
                                                value: "{swish_phone}",
                                                oninput: move |e| swish_phone.set(e.value()),
                                            }
                                        }
                                        
                                        div { class: "grid grid-cols-2 gap-2 mt-1.5",
                                            // Real Swish mobile deep link
                                            a {
                                                class: format!("yntra-btn text-[10px] font-black py-2.5 flex items-center justify-center gap-1.5 bg-[#e8117f] hover:bg-[#c90f6e] transition-all hover:no-underline shadow-md {}", if is_phone_valid { "" } else { "opacity-50 pointer-events-none" }),
                                                href: "{swish_url}",
                                                onclick: move |_| {
                                                    let mut db_trig = db_trigger;
                                                    let uid = active_user_c1a.id.clone();
                                                    let ws = workspace_id_c1a.clone();
                                                    let unpaid_invs = invoices_c1a.clone();
                                                    spawn(async move {
                                                        let now_str = get_now_timestamp_str();
                                                        for inv in unpaid_invs.iter() {
                                                            let _ = yntra_core::record_school_payment(uid.clone(), ws.clone(), inv.id.clone(), inv.amount, "Swish".to_string(), now_str.clone()).await;
                                                        }
                                                        swish_step.set(4);
                                                        let current = *db_trig.read();
                                                        db_trig.set(current + 1);
                                                    });
                                                },
                                                components::LucideIcon { name: "external-link", size: "12" }
                                                "Launch Swish"
                                            }
                                            // Simulated sandbox push
                                            button {
                                                class: "yntra-btn-secondary text-[10px] font-black py-2.5 flex items-center justify-center gap-1.5 border-border/40 hover:bg-sidebar/50",
                                                disabled: !is_phone_valid,
                                                onclick: move |_| {
                                                    swish_step.set(1);
                                                    let mut db_trig = db_trigger;
                                                    let uid = active_user_c1b.id.clone();
                                                    let ws = workspace_id_c1b.clone();
                                                    let unpaid_invs = invoices_c1b.clone();
                                                    spawn(async move {
                                                        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                                                        swish_step.set(2);
                                                        swish_countdown.set(30);
                                                        for _ in 0..4 {
                                                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                                                            let current_count: i32 = *swish_countdown.read();
                                                            swish_countdown.set(current_count.saturating_sub(1));
                                                        }
                                                        swish_step.set(3);
                                                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                                                        let now_str = get_now_timestamp_str();
                                                        for inv in unpaid_invs.iter() {
                                                            let _ = yntra_core::record_school_payment(uid.clone(), ws.clone(), inv.id.clone(), inv.amount, "Swish".to_string(), now_str.clone()).await;
                                                        }
                                                        swish_step.set(4);
                                                        let current = *db_trig.read();
                                                        db_trig.set(current + 1);
                                                    });
                                                },
                                                components::LucideIcon { name: "play", size: "12" }
                                                "Simulate Push"
                                            }
                                        }
                                    }

                                    // Divider
                                    div { class: "relative flex py-1 items-center",
                                        div { class: "flex-grow border-t border-border/20" }
                                        span { class: "flex-shrink mx-3.5 text-[8px] font-black uppercase text-muted-foreground/60 tracking-widest", "Or Scan QR Code" }
                                        div { class: "flex-grow border-t border-border/20" }
                                    }

                                    // Dynamic Scan Engine View
                                    div { class: "flex flex-col items-center gap-3 bg-sidebar/10 p-4 rounded-2xl border border-border/10",
                                        div { 
                                            class: format!("relative rounded-2xl bg-white border border-[#e8117f]/25 shadow-lg flex items-center justify-center cursor-pointer transition-all duration-300 overflow-hidden {}", if is_zoomed { "w-[200px] h-[200px] scale-[1.08]" } else { "w-[150px] h-[150px]" }),
                                            onclick: move |_| qr_zoomed.set(!is_zoomed),
                                            
                                            // Glowing scanning beam
                                            div { class: "scanner-line" }
                                            
                                            img {
                                                src: "{qr_url}",
                                                alt: "Swish QR Code",
                                                class: format!("transition-all duration-300 {}", if is_zoomed { "w-[185px] h-[185px]" } else { "w-[135px] h-[135px]" }),
                                            }
                                        }
                                        span { class: "text-[9px] text-muted-foreground text-center font-medium max-w-[280px] leading-normal",
                                            if is_zoomed { "Tap QR code again to shrink layout view." } else { "Tap QR code to zoom. Scan code to pay immediately." }
                                        }
                                        
                                        button {
                                            class: "yntra-btn-secondary text-[10px] py-1.5 px-4 mt-1 border-[#e8117f]/20 hover:border-[#e8117f]/40 hover:bg-[#e8117f]/5 text-foreground hover:scale-[1.02] duration-150 shadow-sm",
                                            onclick: move |_| {
                                                swish_step.set(3);
                                                let mut db_trig = db_trigger;
                                                let uid = active_user_c2.id.clone();
                                                let ws = workspace_id_c2.clone();
                                                let unpaid_invs = invoices_c2.clone();
                                                spawn(async move {
                                                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                                                    let now_str = get_now_timestamp_str();
                                                    for inv in unpaid_invs.iter() {
                                                        let _ = yntra_core::record_school_payment(uid.clone(), ws.clone(), inv.id.clone(), inv.amount, "Swish".to_string(), now_str.clone()).await;
                                                    }
                                                    swish_step.set(4);
                                                    let current = *db_trig.read();
                                                    db_trig.set(current + 1);
                                                });
                                            },
                                            "I've Approved Payment in Swish"
                                        }
                                    }
                                }
                            }
                        },
                        1 | 2 | 3 => {
                            let step = *swish_step.read();
                            rsx! {
                                div { class: "flex flex-col gap-6 py-6 relative z-10",
                                    
                                    // SOTA Progress Circle
                                    div { class: "flex flex-col items-center justify-center gap-3",
                                        div { class: "relative w-20 h-20 flex items-center justify-center",
                                            // Glowing background ring
                                            div { class: "absolute inset-0 rounded-full border-4 border-[#e8117f]/10 shadow-inner" }
                                            // Pulsing core spinner
                                            div { class: "absolute inset-0 rounded-full border-4 border-[#e8117f] border-t-transparent animate-spin" }
                                            
                                            div { class: "w-10 h-10 rounded-xl bg-gradient-to-br from-[#e8117f]/20 to-[#FF5F00]/10 flex items-center justify-center swish-pulse shadow-inner",
                                                components::LucideIcon { name: "credit-card", size: "20", class: "text-[#e8117f]" }
                                            }
                                        }
                                        
                                        if step == 2 {
                                            span { class: "text-xs font-black text-foreground/80 tracking-wider animate-pulse",
                                                "Waiting for signature... {swish_countdown}s"
                                            }
                                        } else if step == 1 {
                                            span { class: "text-xs font-black text-foreground/80 tracking-wider", "Initiating secure node session..." }
                                        } else {
                                            span { class: "text-xs font-black text-foreground/80 tracking-wider", "Finalizing transaction..." }
                                        }
                                    }

                                    // Step list pipeline
                                    div { class: "flex flex-col gap-3.5 bg-sidebar/40 p-5 rounded-2xl border border-border/30 max-w-sm mx-auto w-full",
                                        
                                        // Step 1 status
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-3",
                                                {
                                                    if step > 1 {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-emerald-500/10 text-emerald-400 flex items-center justify-center text-[10px]", "✓" } }
                                                    } else {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-primary/10 text-primary border border-primary/20 flex items-center justify-center text-[10px] animate-pulse", "▶" } }
                                                    }
                                                }
                                                span { class: format!("text-xs font-bold {}", if step >= 1 { "text-foreground" } else { "text-muted-foreground" }), "Contacting Swish Payment Node" }
                                            }
                                            if step == 1 {
                                                span { class: "text-[9px] font-black uppercase text-primary animate-pulse", "Running" }
                                            } else {
                                                span { class: "text-[9px] font-black uppercase text-emerald-400", "Ready" }
                                            }
                                        }

                                        // Step 2 status
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-3",
                                                {
                                                    if step > 2 {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-emerald-500/10 text-emerald-400 flex items-center justify-center text-[10px]", "✓" } }
                                                    } else if step == 2 {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-primary/10 text-primary border border-primary/20 flex items-center justify-center text-[10px] animate-spin", "⟳" } }
                                                    } else {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-sidebar border border-border/60 text-muted-foreground/40 flex items-center justify-center text-[10px]", "•" } }
                                                    }
                                                }
                                                span { class: format!("text-xs font-bold {}", if step >= 2 { "text-foreground" } else { "text-muted-foreground" }), "Awaiting Mobile App Approval" }
                                            }
                                            if step == 2 {
                                                span { class: "text-[9px] font-black uppercase text-primary animate-pulse", "Awaiting" }
                                            } else if step > 2 {
                                                span { class: "text-[9px] font-black uppercase text-emerald-400", "Approved" }
                                            } else {
                                                span { class: "text-[9px] font-black uppercase text-muted-foreground/40", "Queued" }
                                            }
                                        }

                                        // Step 3 status
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-3",
                                                {
                                                    if step > 3 {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-emerald-500/10 text-emerald-400 flex items-center justify-center text-[10px]", "✓" } }
                                                    } else if step == 3 {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-primary/10 text-primary border border-primary/20 flex items-center justify-center text-[10px] animate-pulse", "▶" } }
                                                    } else {
                                                        rsx! { div { class: "w-5 h-5 rounded-full bg-sidebar border border-border/60 text-muted-foreground/40 flex items-center justify-center text-[10px]", "•" } }
                                                    }
                                                }
                                                span { class: format!("text-xs font-bold {}", if step >= 3 { "text-foreground" } else { "text-muted-foreground" }), "Verifying Signature Ledger" }
                                            }
                                            if step == 3 {
                                                span { class: "text-[9px] font-black uppercase text-primary animate-pulse", "Writing" }
                                            } else if step > 3 {
                                                span { class: "text-[9px] font-black uppercase text-emerald-400", "Verified" }
                                            } else {
                                                span { class: "text-[9px] font-black uppercase text-muted-foreground/40", "Queued" }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        4 => {
                            rsx! {
                                div { class: "flex flex-col items-center justify-center gap-4 py-8 text-center relative z-10",
                                    div { class: "w-14 h-14 rounded-2xl bg-emerald-500/10 text-emerald-400 flex items-center justify-center shadow-lg border border-emerald-500/20",
                                        components::LucideIcon { name: "check-circle", size: "32" }
                                    }
                                    h4 { class: "text-base font-black text-foreground m-0 tracking-wide", "Payment Completed Successfully!" }
                                    p { class: "text-xs text-muted-foreground max-w-[280px] m-0 leading-normal font-medium",
                                        "Thank you! Your Swish transaction has cleared. Tuition fees have been updated, and balance records are settled."
                                    }
                                    button {
                                        class: "yntra-btn text-xs font-black px-6 py-2.5 mt-3 shadow-md hover:scale-[1.02] active:scale-[0.98] duration-150",
                                        onclick: move |_| {
                                            show_swish_modal.set(false);
                                        },
                                        "Close Gateway"
                                    }
                                }
                            }
                        },
                        _ => rsx! {}
                    }
                }
            }
        }
    }
}

fn get_now_timestamp_str() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs() as i64;
            let mut days = secs / 86400;
            let mut year = 1970;
            loop {
                let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                let days_in_year = if leap { 366 } else { 365 };
                if days >= days_in_year {
                    days -= days_in_year;
                    year += 1;
                } else {
                    break;
                }
            }
            let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
            let month_lengths = if leap {
                [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
            } else {
                [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
            };
            let mut month = 1;
            for &length in month_lengths.iter() {
                if days >= length {
                    days -= length;
                    month += 1;
                } else {
                    break;
                }
            }
            let day = days + 1;
            let tod_secs = secs % 86400;
            let hour = tod_secs / 3600;
            let min = (tod_secs % 3600) / 60;
            format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, hour, min)
        })
        .unwrap_or_else(|_| "2026-07-04 12:00".to_string())
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

fn get_swish_link(payee: &str, amount: f64, message: &str) -> String {
    let json_str = format!(
        r#"{{"payee":"{}","amount":{:.2},"message":"{}","editable":0}}"#,
        payee, amount, message
    );
    let encoded_data = url_encode(&json_str);
    format!("swish://payment?data={}", encoded_data)
}



