use dioxus::prelude::*;

#[component]
pub fn DispatchAlertsModal(
    job_id: String,
    customer_id: Option<String>,
    active_user_id: String,
    on_close: EventHandler<()>,
) -> Element {
    let mut eta_input = use_signal(|| "15".to_string());
    let mut feedback_msg = use_signal(|| Option::<String>::None);
    let mut is_sending = use_signal(|| false);

    let cust_id = customer_id.unwrap_or_else(|| "cust-default".to_string());

    let j_id_1 = job_id.clone();
    let u_id_1 = active_user_id.clone();
    let c_id_1 = cust_id.clone();

    let j_id_2 = job_id.clone();
    let u_id_2 = active_user_id.clone();
    let c_id_2 = cust_id.clone();

    let j_id_3 = job_id;
    let u_id_3 = active_user_id;
    let c_id_3 = cust_id;

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-4 font-sans text-slate-100 animate-in fade-in duration-200",

            div {
                class: "bg-slate-900 border border-slate-700/80 rounded-2xl w-full max-w-xl flex flex-col shadow-2xl overflow-hidden",

                // Modal Header
                div {
                    class: "px-6 py-4 bg-slate-950 border-b border-slate-800 flex items-center justify-between",
                    div {
                        class: "flex items-center gap-3",
                        span { class: "text-3xl", "💬" }
                        div {
                            h2 { class: "text-xl font-black text-white tracking-wide uppercase", "AUTOMATISERADE SMS & DISPATCH-ALERTER" }
                            p { class: "text-xs text-slate-400 font-mono", "UPPDRAG: #{j_id_1} • TWILIO / PLIVO / WHATSAPP / SENDGRID" }
                        }
                    }
                    button {
                        class: "h-9 w-9 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-lg flex items-center justify-center font-bold text-lg transition-all",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Active Gateway Badges
                div {
                    class: "px-6 py-3 bg-slate-950/60 border-b border-slate-800 flex items-center gap-3 text-xs font-mono",
                    span { class: "text-slate-400 font-bold", "GATEWAYS:" }
                    div { class: "px-2 py-0.5 bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 rounded font-semibold", "✓ PLIVO SMS" }
                    div { class: "px-2 py-0.5 bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 rounded font-semibold", "✓ TWILIO SMS" }
                    div { class: "px-2 py-0.5 bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 rounded font-semibold", "✓ WHATSAPP META" }
                    div { class: "px-2 py-0.5 bg-primary/20 text-primary border border-primary/30 rounded font-semibold", "✓ SENDGRID EMAIL" }
                }

                // Feedback Banner
                if let Some(ref msg) = *feedback_msg.read() {
                    div {
                        class: "mx-6 mt-4 p-3 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl text-xs font-mono font-bold flex items-center justify-between",
                        span { "✓ {msg}" }
                        button { class: "text-emerald-400 font-bold ml-2", onclick: move |_| feedback_msg.set(None), "✕" }
                    }
                }

                // Content Body
                div {
                    class: "p-6 space-y-5",

                    div {
                        label { class: "block text-xs text-slate-400 mb-1 font-medium", "Beräknad Ankomsttid / ETA (Minuter)" }
                        input {
                            class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-primary outline-none",
                            value: "{eta_input}",
                            oninput: move |e| eta_input.set(e.value())
                        }
                    }

                    // Dispatch Alert Actions
                    div {
                        class: "space-y-3 pt-2",

                        button {
                            class: "w-full py-3.5 bg-emerald-600 hover:bg-emerald-500 active:scale-[0.99] text-white font-bold text-sm rounded-xl shadow-lg transition-all border border-emerald-400 flex items-center justify-center gap-3",
                            disabled: *is_sending.read(),
                            onclick: move |_| {
                                let j_id = j_id_1.clone();
                                let u_id = u_id_1.clone();
                                let c_id = c_id_1.clone();
                                let mins = eta_input.read().parse::<u32>().ok();
                                is_sending.set(true);

                                spawn(async move {
                                    if let Ok(ok) = yntra_core::send_driver_en_route_alert(u_id, j_id, c_id, mins).await {
                                        if ok {
                                            feedback_msg.set(Some("Förare på väg SMS & WhatsApp har skickats till kunden!".to_string()));
                                        }
                                    }
                                    is_sending.set(false);
                                });
                            },
                            span { class: "text-lg", "🚚" }
                            span { "SKICKA 'FÖRARE PÅ VÄG' ALERT (SMS / WHATSAPP)" }
                        }

                        button {
                            class: "w-full py-3.5 bg-primary hover:bg-primary active:scale-[0.99] text-white font-bold text-sm rounded-xl shadow-lg transition-all border border-primary/40 flex items-center justify-center gap-3",
                            disabled: *is_sending.read(),
                            onclick: move |_| {
                                let j_id = j_id_2.clone();
                                let u_id = u_id_2.clone();
                                let c_id = c_id_2.clone();
                                let mins = eta_input.read().parse::<u32>().unwrap_or(10);
                                is_sending.set(true);

                                spawn(async move {
                                    if let Ok(ok) = yntra_core::send_live_eta_update_alert(u_id, j_id, c_id, mins).await {
                                        if ok {
                                            feedback_msg.set(Some(format!("Live ETA uppdatering ({mins} min) skickad via SMS!")));
                                        }
                                    }
                                    is_sending.set(false);
                                });
                            },
                            span { class: "text-lg", "⏱️" }
                            span { "SKICKA LIVE ETA UPPDATERING VIA SMS" }
                        }

                        button {
                            class: "w-full py-3.5 bg-amber-600 hover:bg-amber-500 active:scale-[0.99] text-white font-black text-sm rounded-xl shadow-lg transition-all border border-amber-400 flex items-center justify-center gap-3",
                            disabled: *is_sending.read(),
                            onclick: move |_| {
                                let j_id = j_id_3.clone();
                                let u_id = u_id_3.clone();
                                let c_id = c_id_3.clone();
                                is_sending.set(true);

                                spawn(async move {
                                    if let Ok(ok) = yntra_core::send_post_move_review_request(u_id, j_id, c_id).await {
                                        if ok {
                                            feedback_msg.set(Some("Automatisk begäran om Google & Yelp omdöme skickad!".to_string()));
                                        }
                                    }
                                    is_sending.set(false);
                                });
                            },
                            span { class: "text-lg", "⭐" }
                            span { "AUTOMATISK BEGÄRAN OM KUNDOMDÖME (GOOGLE / YELP)" }
                        }
                    }
                }
            }
        }
    }
}
