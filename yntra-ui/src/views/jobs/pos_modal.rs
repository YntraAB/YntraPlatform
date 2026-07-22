use dioxus::prelude::*;
use crate::components;

#[component]
pub fn PosTerminalModal(
    active_user_id: String,
    invoice_id: String,
    amount_sek: f64,
    db_trigger: Signal<u32>,
    on_close: EventHandler<()>,
) -> Element {
    let toast = dioxus_primitives::toast::use_toast();
    let mut reader_type = use_signal(|| "stripe_terminal".to_string());
    let mut is_connecting = use_signal(|| false);
    let mut session_data = use_signal(|| Option::<yntra_core::MobilePosTerminalSession>::None);
    let mut is_processing = use_signal(|| false);
    let mut card_brand_input = use_signal(|| "Visa".to_string());
    let mut last4_input = use_signal(|| "4242".to_string());
    let mut error_msg = use_signal(|| Option::<String>::None);

    let handle_connect_reader = {
        let active_uid = active_user_id.clone();
        let inv_id = invoice_id.clone();
        move |_| {
            let uid = active_uid.clone();
            let iid = inv_id.clone();
            let prov = reader_type.read().clone();
            is_connecting.set(true);
            error_msg.set(None);
            spawn(async move {
                let res = yntra_core::initiate_mobile_pos_terminal_session(
                    uid,
                    iid,
                    Some(prov),
                    Some("reader_bt_field_01".to_string()),
                ).await;
                is_connecting.set(false);
                match res {
                    Ok(sess) => {
                        session_data.set(Some(sess));
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Läsarnanslutning misslyckades: {}", e)));
                    }
                }
            });
        }
    };

    let handle_confirm_payment = {
        let active_uid = active_user_id.clone();
        let inv_id = invoice_id.clone();
        let mut d_trig = db_trigger;
        move |_| {
            let uid = active_uid.clone();
            let iid = inv_id.clone();
            let brand = card_brand_input.read().clone();
            let l4 = last4_input.read().clone();
            is_processing.set(true);
            error_msg.set(None);
            spawn(async move {
                let res = yntra_core::confirm_mobile_pos_terminal_payment(
                    uid,
                    iid,
                    "card_present_tap".to_string(),
                    brand,
                    l4,
                ).await;
                is_processing.set(false);
                match res {
                    Ok(_) => {
                        toast.success(
                            "Kortbetalning godkänd via On-Site Kortterminal!".to_string(),
                            dioxus_primitives::toast::ToastOptions::new(),
                        );
                        let current = *d_trig.read();
                        d_trig.set(current + 1);
                        on_close.call(());
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Kortbetalning nekad: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4 animate-in fade-in duration-200",
            onclick: move |_| on_close.call(()),

            div {
                class: "bg-background border border-border rounded-2xl shadow-2xl w-full max-w-md overflow-hidden flex flex-col max-h-[90vh]",
                onclick: move |e| e.stop_propagation(),

                // Modal Header
                div { class: "p-4 border-b border-border bg-muted/40 flex justify-between items-center",
                    div { class: "flex items-center gap-2",
                        div { class: "p-2 rounded-lg bg-emerald-500/10 text-emerald-500 border border-emerald-500/20",
                            components::LucideIcon { name: "credit-card", size: "18" }
                        }
                        div {
                            h3 { class: "text-sm font-extrabold text-foreground m-0", "Fältbetalning - Kortläsare / POS" }
                            p { class: "text-[10px] text-muted-foreground m-0", "Ta emot kortbetalning på plats innan urlastning" }
                        }
                    }
                    button {
                        class: "p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer border-0 bg-transparent",
                        onclick: move |_| on_close.call(()),
                        components::LucideIcon { name: "x", size: "16" }
                    }
                }

                // Modal Body
                div { class: "p-5 space-y-4 overflow-y-auto flex-1 text-xs",
                    
                    // Amount Banner
                    div { class: "bg-emerald-500/10 border border-emerald-500/20 rounded-xl p-3.5 flex justify-between items-center",
                        div {
                            div { class: "text-[10px] font-bold text-emerald-500 uppercase tracking-wider", "Belopp att debitera" }
                            div { class: "text-lg font-black text-foreground mt-0.5", "{amount_sek:.2} SEK" }
                        }
                        span { class: "text-[10px] font-bold bg-emerald-500/20 text-emerald-400 px-2.5 py-1 rounded-full border border-emerald-500/30",
                            "Kort & Tap-to-Pay"
                        }
                    }

                    if let Some(ref sess) = *session_data.read() {
                        // Live Card Reader Terminal Session Active View
                        div { class: "space-y-4 bg-muted/30 border border-border/60 rounded-xl p-4 animate-in fade-in duration-200",
                            div { class: "flex items-center justify-between border-b border-border/40 pb-2.5",
                                div { class: "flex items-center gap-2 text-emerald-400 font-bold text-xs",
                                    span { class: "h-2 w-2 rounded-full bg-emerald-400 animate-pulse" }
                                    "Kortläsare Redo"
                                }
                                span { class: "font-mono text-[9px] text-muted-foreground bg-background px-2 py-0.5 rounded border border-border/40",
                                    "{sess.session_id}"
                                }
                            }

                            div { class: "text-center py-3 space-y-1.5",
                                components::LucideIcon { name: "credit-card", class: "h-10 w-10 mx-auto text-primary animate-bounce" }
                                p { class: "font-black text-sm text-foreground m-0", "Blipp / Sätt i kortet nu" }
                                p { class: "text-[10px] text-muted-foreground m-0", "Terminal väntar på kundens kortbetalning..." }
                            }

                            // Card Brand and Details selector for field confirmation
                            div { class: "grid grid-cols-2 gap-2 pt-2 border-t border-border/30",
                                div { class: "flex flex-col gap-1",
                                    label { class: "text-[9px] font-bold text-muted-foreground uppercase", "Korttyp" }
                                    select {
                                        class: "px-2 py-1 text-xs border border-border bg-background rounded text-foreground",
                                        value: "{card_brand_input}",
                                        onchange: move |e| card_brand_input.set(e.value()),
                                        option { value: "Visa", "Visa" }
                                        option { value: "Mastercard", "Mastercard" }
                                        option { value: "Amex", "American Express" }
                                        option { value: "ApplePay", "Apple Pay / Google Pay" }
                                    }
                                }
                                div { class: "flex flex-col gap-1",
                                    label { class: "text-[9px] font-bold text-muted-foreground uppercase", "Sista 4 siffror" }
                                    input {
                                        r#type: "text",
                                        maxlength: "4",
                                        class: "px-2 py-1 text-xs border border-border bg-background rounded text-foreground font-mono",
                                        value: "{last4_input}",
                                        oninput: move |e| last4_input.set(e.value())
                                    }
                                }
                            }

                            button {
                                class: "w-full py-2.5 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-xs cursor-pointer shadow transition-all border-0 flex items-center justify-center gap-2 mt-2",
                                disabled: *is_processing.read(),
                                onclick: handle_confirm_payment,
                                if *is_processing.read() {
                                    components::LucideIcon { name: "loader-2", class: "animate-spin h-4 w-4" }
                                    "Behandlar Transaktion..."
                                } else {
                                    components::LucideIcon { name: "check-circle", size: "14" }
                                    "Bekräfta Genomförd Kortbetalning"
                                }
                            }
                        }
                    } else {
                        // Hardware Selection View
                        div { class: "space-y-3",
                            label { class: "text-[11px] font-bold text-foreground block", "Välj Kortläsare / Terminal" }
                            
                            div { class: "grid grid-cols-2 gap-2",
                                button {
                                    class: format!(
                                        "p-3 rounded-xl border text-left cursor-pointer transition-all flex flex-col justify-between gap-2 {}",
                                        if *reader_type.read() == "stripe_terminal" { "border-primary bg-primary/10 text-foreground" } else { "border-border bg-background hover:bg-muted text-muted-foreground" }
                                    ),
                                    onclick: move |_| reader_type.set("stripe_terminal".to_string()),
                                    components::LucideIcon { name: "radio", size: "16" }
                                    div {
                                        div { class: "font-bold text-xs", "Stripe Terminal" }
                                        div { class: "text-[9px] opacity-70", "Bordsterminal & Bluetooth" }
                                    }
                                }

                                button {
                                    class: format!(
                                        "p-3 rounded-xl border text-left cursor-pointer transition-all flex flex-col justify-between gap-2 {}",
                                        if *reader_type.read() == "tap_to_pay" { "border-primary bg-primary/10 text-foreground" } else { "border-border bg-background hover:bg-muted text-muted-foreground" }
                                    ),
                                    onclick: move |_| reader_type.set("tap_to_pay".to_string()),
                                    components::LucideIcon { name: "smartphone", size: "16" }
                                    div {
                                        div { class: "font-bold text-xs", "Tap to Pay" }
                                        div { class: "text-[9px] opacity-70", "NFC kontaktlöst i mobil" }
                                    }
                                }
                            }

                            button {
                                class: "w-full py-2.5 rounded-xl bg-primary text-primary-foreground font-bold text-xs cursor-pointer shadow hover:opacity-90 transition-all border-0 flex items-center justify-center gap-2 mt-2",
                                disabled: *is_connecting.read(),
                                onclick: handle_connect_reader,
                                if *is_connecting.read() {
                                    components::LucideIcon { name: "loader-2", class: "animate-spin h-4 w-4" }
                                    "Ansluter till läsare..."
                                } else {
                                    components::LucideIcon { name: "wifi", size: "14" }
                                    "Koppla upp Kortläsare"
                                }
                            }
                        }
                    }

                    if let Some(ref err) = error_msg.read().as_ref() {
                        div { class: "bg-red-500/10 border border-red-500/20 text-red-500 rounded-xl p-3 text-[10px] font-bold text-center",
                            "⚠️ {err}"
                        }
                    }
                }

                // Modal Footer
                div { class: "p-4 border-t border-border bg-muted/20 flex justify-end",
                    button {
                        class: "px-4 py-2 rounded-lg bg-muted hover:bg-accent text-foreground text-xs font-medium cursor-pointer border border-border transition-colors",
                        onclick: move |_| on_close.call(()),
                        "Stäng"
                    }
                }
            }
        }
    }
}
