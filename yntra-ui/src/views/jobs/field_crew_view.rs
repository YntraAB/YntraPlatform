use dioxus::prelude::*;

use yntra_core::JobTicket;

#[derive(Props, Clone, PartialEq)]
pub struct FieldCrewViewProps {
    pub job_id: String,
    pub job: Option<JobTicket>,
    pub active_user_id: String,
    pub on_close: EventHandler<()>,
    pub on_open_condition_modal: Option<EventHandler<()>>,
    pub on_open_pos_modal: Option<EventHandler<()>>,
}

#[component]
pub fn FieldCrewView(props: FieldCrewViewProps) -> Element {
    let job_id = props.job_id.clone();
    let on_close_handler = props.on_close;
    let job_prop = props.job.clone();

    let mut current_status = use_signal(|| "LASTNING_PÅGÅR".to_string());
    let mut captured_photos = use_signal(|| vec![
        "Foto 1: Trapphus damage waiver".to_string(),
        "Foto 2: Spegel inslagen filt".to_string(),
    ]);
    let mut is_offline = use_signal(|| true); // Field environment default: basement / low-connectivity
    let mut offline_queue_count = use_signal(|| 3u32);
    let mut flash_feedback = use_signal(|| Option::<String>::None);

    let j_id = job_id.clone();
    let tracking_res = use_resource(move || {
        let id_val = j_id.clone();
        async move {
            yntra_core::get_customer_live_tracking_portal(id_val).await.ok()
        }
    });

    let portal_opt = tracking_res.read().clone().flatten();

    let (job_title_display, origin_address_display, destination_address_display) = if let Some(ref j) = job_prop {
        (
            j.title.clone(),
            j.origin_address.clone().unwrap_or_else(|| j.location_address.clone()),
            j.destination_address.clone().unwrap_or_else(|| "Ej angiven".to_string()),
        )
    } else if let Some(ref p) = portal_opt {
        (
            format!("Flyttuppdrag #{}", p.job_ticket_id),
            p.origin_address.clone(),
            p.destination_address.clone(),
        )
    } else {
        (
            "Laddar uppdrag...".to_string(),
            "Laddar adress...".to_string(),
            "Laddar adress...".to_string(),
        )
    };

    let driver_phone_display = portal_opt.as_ref().and_then(|p| p.driver_phone.clone());

    let on_open_condition = props.on_open_condition_modal.clone();
    let on_open_pos = props.on_open_pos_modal.clone();

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black flex flex-col font-sans select-none text-yellow-300 animate-in fade-in duration-150 p-2 sm:p-4 overflow-y-auto",

            // TOP NAVIGATION & OFFLINE STATUS HEADER (Tactile & Ultra High Contrast)
            div {
                class: "w-full bg-slate-950 border-4 border-yellow-400 rounded-2xl p-3 sm:p-4 flex items-center justify-between shadow-2xl mb-3",
                div {
                    class: "flex items-center gap-3",
                    button {
                        class: "h-16 px-6 bg-red-600 hover:bg-red-500 active:scale-95 text-white font-extrabold text-xl rounded-xl border-2 border-white shadow-lg flex items-center justify-center transition-all",
                        onclick: move |_| on_close_handler.call(()),
                        "✕ STÄNG"
                    }
                    div {
                        div { class: "text-2xl font-black tracking-wider text-white uppercase", "FÄLTLÄGE / HANDSKE" }
                        div { class: "text-base font-bold text-yellow-400 font-mono", "UPPDRAG: #{job_id}" }
                    }
                }

                // Low-Connectivity / Offline Queue Badge
                div {
                    class: "flex items-center gap-2 bg-slate-900 border-2 border-yellow-500/80 px-4 py-2 rounded-xl",
                    button {
                        class: if *is_offline.read() {
                            "px-3 py-1 bg-amber-500 text-black font-extrabold text-sm rounded-lg animate-pulse"
                        } else {
                            "px-3 py-1 bg-emerald-500 text-black font-extrabold text-sm rounded-lg"
                        },
                        onclick: move |_| {
                            let curr = *is_offline.read();
                            is_offline.set(!curr);
                        },
                        if *is_offline.read() { "OFFLINE-LÄGE (KÄLLARE)" } else { "ONLINE (4G/5G)" }
                    }
                    div { class: "text-sm font-bold text-white font-mono",
                        "KÖ: {offline_queue_count.read()} KR-ÄNDRINGAR"
                    }
                }
            }

            // FLASH FEEDBACK BANNER
            if let Some(msg) = flash_feedback.read().clone() {
                div {
                    class: "w-full bg-emerald-500 text-black font-black text-2xl text-center py-3 rounded-xl border-4 border-white animate-bounce mb-3 shadow-2xl",
                    "✓ {msg}"
                }
            }

            // MAIN FIELD WORKFLOW GRID (64px+ Extra Large Buttons)
            div {
                class: "flex-1 grid grid-cols-1 md:grid-cols-2 gap-4 mb-4",

                // LEFT PANEL: 1-TAP STATUS TRANSITIONS & JOB INFO
                div {
                    class: "bg-slate-950 border-4 border-yellow-400 rounded-2xl p-4 flex flex-col justify-between space-y-4 shadow-2xl",
                    
                    div {
                        class: "space-y-2 border-b-2 border-yellow-500/40 pb-3",
                        div { class: "text-xs font-black uppercase text-yellow-500 tracking-widest", "KUND & ADRESS" }
                        div { class: "text-2xl font-extrabold text-white", "{job_title_display}" }
                        div { class: "text-lg font-bold text-yellow-300", "📍 Från: {origin_address_display}" }
                        div { class: "text-lg font-bold text-emerald-400", "🏁 Till: {destination_address_display}" }
                    }

                    // Quick Status Selector (1-Tap Large Targets)
                    div {
                        class: "space-y-3",
                        div { class: "text-xs font-black uppercase text-yellow-500 tracking-widest", "STATUSUPPDATERING (1-TRYCK)" }
                        
                        div {
                            class: "grid grid-cols-2 gap-3",
                            
                            button {
                                class: if *current_status.read() == "LASTNING_PÅGÅR" {
                                    "h-20 bg-yellow-400 text-black font-black text-xl rounded-xl border-4 border-white shadow-xl active:scale-95 transition-all flex items-center justify-center"
                                } else {
                                    "h-20 bg-slate-900 text-yellow-300 font-bold text-lg rounded-xl border-2 border-yellow-500/60 hover:bg-slate-800 active:scale-95 transition-all flex items-center justify-center"
                                },
                                onclick: move |_| {
                                    current_status.set("LASTNING_PÅGÅR".to_string());
                                    let next_cnt = *offline_queue_count.read() + 1;
                                    offline_queue_count.set(next_cnt);
                                    flash_feedback.set(Some("LASTNING PÅBÖRJAD".to_string()));
                                },
                                "📦 BÖRJA LASTNING"
                            }

                            button {
                                class: if *current_status.read() == "I_TRANSIT" {
                                    "h-20 bg-amber-500 text-black font-black text-xl rounded-xl border-4 border-white shadow-xl active:scale-95 transition-all flex items-center justify-center"
                                } else {
                                    "h-20 bg-slate-900 text-amber-400 font-bold text-lg rounded-xl border-2 border-amber-500/60 hover:bg-slate-800 active:scale-95 transition-all flex items-center justify-center"
                                },
                                onclick: move |_| {
                                    current_status.set("I_TRANSIT".to_string());
                                    let next_cnt = *offline_queue_count.read() + 1;
                                    offline_queue_count.set(next_cnt);
                                    flash_feedback.set(Some("I TRANSIT MOT DESTINATION".to_string()));
                                },
                                "🚚 I TRANSIT"
                            }

                            button {
                                class: if *current_status.read() == "ANLÄNT" {
                                    "h-20 bg-blue-500 text-white font-black text-xl rounded-xl border-4 border-white shadow-xl active:scale-95 transition-all flex items-center justify-center"
                                } else {
                                    "h-20 bg-slate-900 text-blue-300 font-bold text-lg rounded-xl border-2 border-blue-500/60 hover:bg-slate-800 active:scale-95 transition-all flex items-center justify-center"
                                },
                                onclick: move |_| {
                                    current_status.set("ANLÄNT".to_string());
                                    let next_cnt = *offline_queue_count.read() + 1;
                                    offline_queue_count.set(next_cnt);
                                    flash_feedback.set(Some("ANLÄNT TILL ADRESS".to_string()));
                                },
                                "📍 ANLÄNT TILL MÅL"
                            }

                            button {
                                class: if *current_status.read() == "SLUT FÖRD" {
                                    "h-20 bg-emerald-500 text-black font-black text-xl rounded-xl border-4 border-white shadow-xl active:scale-95 transition-all flex items-center justify-center"
                                } else {
                                    "h-20 bg-slate-900 text-emerald-300 font-bold text-lg rounded-xl border-2 border-emerald-500/60 hover:bg-slate-800 active:scale-95 transition-all flex items-center justify-center"
                                },
                                onclick: move |_| {
                                    current_status.set("SLUTFÖRD".to_string());
                                    let next_cnt = *offline_queue_count.read() + 1;
                                    offline_queue_count.set(next_cnt);
                                    flash_feedback.set(Some("FLYTTUPPDRAG SLUTFÖRT!".to_string()));
                                },
                                "🏁 SLUTFÖR FLYTT"
                            }
                        }
                    }

                    if let Some(ref phone) = driver_phone_display {
                        a {
                            class: "h-16 w-full bg-emerald-600 hover:bg-emerald-500 active:scale-95 text-white font-black text-xl rounded-xl border-2 border-white shadow-xl flex items-center justify-center gap-3 transition-all",
                            href: "tel:{phone}",
                            "📞 RING DISP / FÖRARE ({phone})"
                        }
                    }
                }

                // RIGHT PANEL: RAPID PHOTO CAPTURE & CONDITION DISPATCH
                div {
                    class: "bg-slate-950 border-4 border-yellow-400 rounded-2xl p-4 flex flex-col justify-between space-y-4 shadow-2xl",
                    
                    div {
                        class: "space-y-2",
                        div { class: "text-xs font-black uppercase text-yellow-500 tracking-widest", "SNABBFOTO & SKADEINSPEKTION" }
                        
                        // Ultra-Large Rapid Shutter Trigger
                        button {
                            class: "h-28 w-full bg-yellow-400 hover:bg-yellow-300 active:scale-95 text-black font-black text-2xl rounded-2xl border-4 border-white shadow-2xl flex items-center justify-center gap-4 transition-all",
                            onclick: move |_| {
                                let photo_idx = captured_photos.read().len() + 1;
                                captured_photos.write().push(format!("Foto {photo_idx}: Fältbild tagen {}", chrono::Utc::now().format("%H:%M:%S")));
                                let next_cnt = *offline_queue_count.read() + 1;
                                offline_queue_count.set(next_cnt);
                                flash_feedback.set(Some(format!("FOTO #{photo_idx} SPARAT LOKALT")));
                            },
                            span { class: "text-4xl", "📷" }
                            span { "TA SNABBFOTO (HANDSKE-KNAPP)" }
                        }
                    }

                    // Photo Queue Preview Strip
                    div {
                        class: "space-y-2 flex-1 overflow-y-auto max-h-48 border-2 border-yellow-500/40 p-3 rounded-xl bg-slate-900",
                        div { class: "text-xs font-bold text-white uppercase", "TAGNA FÄLTBILDER ({captured_photos.read().len()} ST)" }
                        for (idx, photo) in captured_photos.read().iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "p-2 bg-slate-800 border border-yellow-500/30 rounded-lg text-sm font-mono text-yellow-200 flex items-center justify-between",
                                span { "🖼️ {photo}" }
                                span { class: "text-xs text-emerald-400 font-bold", "OK (LOKAL)" }
                            }
                        }
                    }

                    // Field POS & Condition Reporting Action Triggers
                    div {
                        class: "grid grid-cols-2 gap-3",

                        if let Some(cb) = on_open_condition {
                            button {
                                class: "h-16 bg-amber-600 hover:bg-amber-500 active:scale-95 text-white font-extrabold text-lg rounded-xl border-2 border-white shadow-xl flex items-center justify-center transition-all",
                                onclick: move |_| cb.call(()),
                                "⚠️ SKADEPROTOKOLL"
                            }
                        }

                        if let Some(cb) = on_open_pos {
                            button {
                                class: "h-16 bg-indigo-600 hover:bg-indigo-500 active:scale-95 text-white font-extrabold text-lg rounded-xl border-2 border-white shadow-xl flex items-center justify-center transition-all",
                                onclick: move |_| cb.call(()),
                                "💳 FÄLT-KORTLÄSARE"
                            }
                        }
                    }
                }
            }
        }
    }
}
