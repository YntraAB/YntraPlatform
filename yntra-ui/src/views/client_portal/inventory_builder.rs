use dioxus::prelude::*;
use std::collections::HashMap;

#[derive(Props, Clone, PartialEq)]
pub struct InventoryBuilderProps {
    pub active_user_id: String,
    pub job_id: String,
    pub on_inventory_updated: EventHandler<()>,
}

#[component]
pub fn InteractiveSelfServiceInventoryBuilder(props: InventoryBuilderProps) -> Element {
    let mut selected_room = use_signal(|| "Living Room".to_string());
    let mut item_quantities = use_signal(|| HashMap::<String, u32>::new());
    let mut feedback_msg = use_signal(|| Option::<String>::None);
    let mut is_saving = use_signal(|| false);

    let catalog = yntra_core::get_furniture_catalog();

    // Group items by category (room)
    let rooms = vec![
        ("Living Room", "🛋️ Vardagsrum"),
        ("Bedroom", "🛏️ Sovrum"),
        ("Kitchen", "🍳 Kök & Matplats"),
        ("Office", "💼 Kontor & Förråd"),
    ];

    let current_catalog_items: Vec<_> = catalog
        .iter()
        .filter(|item| item.category == *selected_room.read())
        .cloned()
        .collect();

    // Calculate totals in real-time
    let mut total_items = 0u32;
    let mut total_volume_m3 = 0.0f64;
    let mut total_weight_kg = 0.0f64;

    let quantities = item_quantities.read();
    for preset in catalog.iter() {
        if let Some(&qty) = quantities.get(&preset.id) {
            if qty > 0 {
                total_items += qty;
                total_volume_m3 += preset.default_volume_m3 * qty as f64;
                total_weight_kg += preset.default_weight_kg * qty as f64;
            }
        }
    }

    // Determine estimated truck size
    let truck_recommendation = if total_volume_m3 <= 10.0 {
        "🚚 Liten Skåpbil / Lite-Ace (Upp till 10 m³)"
    } else if total_volume_m3 <= 20.0 {
        "🚚 20m³ Flyttbuss (B-kort godkänd)"
    } else if total_volume_m3 <= 35.0 {
        "🚚 35m³ Stor Lastbil"
    } else {
        "🚛 50m³ Tung Lastbil med Släp"
    };

    // Calculate automated price estimate (SEK)
    let base_price = 1500.0;
    let price_per_m3 = 280.0;
    let estimated_cost_sek = if total_items == 0 {
        0.0
    } else {
        base_price + (total_volume_m3 * price_per_m3)
    };

    let cost_min = (estimated_cost_sek * 0.95).round() as u32;
    let cost_max = (estimated_cost_sek * 1.10).round() as u32;

    rsx! {
        div { class: "bg-slate-900 border border-slate-700/80 rounded-2xl p-6 text-slate-100 shadow-2xl space-y-6 font-sans animate-in fade-in duration-200",

            // Top Header & Real-Time Estimator Bar
            div { class: "bg-slate-950 border border-slate-800 rounded-xl p-5 shadow-inner space-y-4",
                div { class: "flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border-b border-slate-800 pb-4",
                    div {
                        h3 { class: "text-xl font-black text-yellow-400 tracking-wide uppercase m-0 flex items-center gap-2",
                            span { "🛋️" }
                            "INTERAKTIV SJÄLVBETJÄNINGS-MÖBELBERÄKNARE"
                        }
                        p { class: "text-xs text-slate-400 m-0 mt-1",
                            "Klicka i möbler rum för rum – få direkt volymberäkning och estimerat flyttpris!"
                        }
                    }

                    // 1-Click Save Button
                    button {
                        class: "px-5 py-3 bg-emerald-600 hover:bg-emerald-500 active:scale-95 text-white font-black text-sm rounded-xl border border-emerald-400 shadow-lg transition-all flex items-center gap-2 cursor-pointer disabled:opacity-50",
                        disabled: *is_saving.read() || total_items == 0,
                        onclick: move |_| {
                            let uid = props.active_user_id.clone();
                            let jid = props.job_id.clone();
                            let items_map = item_quantities.read().clone();
                            let catalog_ref = catalog.clone();
                            let on_updated = props.on_inventory_updated.clone();

                            is_saving.set(true);

                            spawn(async move {
                                for preset in catalog_ref.iter() {
                                    if let Some(&qty) = items_map.get(&preset.id) {
                                        if qty > 0 {
                                            let _ = yntra_core::create_move_inventory_item(
                                                uid.clone(),
                                                jid.clone(),
                                                preset.name.clone(),
                                                preset.category.clone(),
                                                qty as i32,
                                                preset.default_volume_m3,
                                                preset.default_handling_notes.clone(),
                                            ).await;
                                        }
                                    }
                                }

                                // Calculate and lock in official quote
                                let _ = yntra_core::calculate_and_save_move_quote(
                                    uid.clone(),
                                    jid,
                                ).await;

                                feedback_msg.set(Some("✓ Möbelinventarie sparad & instant offert är nu beräknad!".to_string()));
                                is_saving.set(false);
                                on_updated.call(());
                            });
                        },
                        span { "💾" }
                        span { "SPARA INVENTARIE & FÅ OFFERT" }
                    }
                }

                // Live Metrics Bar
                div { class: "grid grid-cols-2 sm:grid-cols-4 gap-3 text-center",
                    div { class: "bg-slate-900 border border-slate-800 p-3 rounded-lg",
                        div { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "TOTALT ANTWAL" }
                        div { class: "text-xl font-black text-white mt-1", "{total_items} st" }
                    }
                    div { class: "bg-slate-900 border border-slate-800 p-3 rounded-lg",
                        div { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "TOTAL VOLYM" }
                        div { class: "text-xl font-black text-indigo-400 mt-1", "{total_volume_m3:.1} m³" }
                    }
                    div { class: "bg-slate-900 border border-slate-800 p-3 rounded-lg",
                        div { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "VIKTUPPSKATTNING" }
                        div { class: "text-xl font-black text-emerald-400 mt-1", "{total_weight_kg:.0} kg" }
                    }
                    div { class: "bg-slate-900 border border-amber-500/40 p-3 rounded-lg bg-amber-500/10",
                        div { class: "text-[10px] font-bold text-amber-400 uppercase tracking-widest", "DIREKTPRIS ESTIMAT" }
                        div { class: "text-xl font-black text-amber-300 mt-1",
                            if total_items > 0 {
                                "{cost_min} kr – {cost_max} kr"
                            } else {
                                "0 kr"
                            }
                        }
                    }
                }

                // Recommended Vehicle Banner
                if total_items > 0 {
                    div { class: "p-3 bg-indigo-950/60 border border-indigo-500/30 rounded-lg text-xs font-mono text-indigo-300 flex items-center justify-between",
                        span { class: "font-bold", "REKOMMENDERAD FLYTTLASTBIL:" }
                        span { class: "font-black text-white", "{truck_recommendation}" }
                    }
                }
            }

            // Feedback Message
            if let Some(ref msg) = *feedback_msg.read() {
                div { class: "p-4 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl text-xs font-mono font-bold flex items-center justify-between",
                    span { "{msg}" }
                    button { class: "text-emerald-400 font-bold ml-2 cursor-pointer", onclick: move |_| feedback_msg.set(None), "✕" }
                }
            }

            // Room Selector Tabs
            div { class: "flex items-center gap-2 overflow-x-auto pb-2 border-b border-slate-800",
                for (room_key , room_label) in rooms.iter() {
                    button {
                        key: "{room_key}",
                        class: if *selected_room.read() == *room_key {
                            "px-4 py-2 bg-indigo-600 text-white font-bold text-xs rounded-xl shadow border border-indigo-400 transition-all cursor-pointer"
                        } else {
                            "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 font-medium text-xs rounded-xl border border-slate-700 transition-all cursor-pointer"
                        },
                        onclick: {
                            let key_str = room_key.to_string();
                            move |_| selected_room.set(key_str.clone())
                        },
                        "{room_label}"
                    }
                }
            }

            // Interactive Furniture Preset Grid
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4",
                for item in current_catalog_items.iter() {
                    {
                        let item_id = item.id.clone();
                        let curr_qty = item_quantities.read().get(&item_id).cloned().unwrap_or(0);
                        let item_id_inc = item_id.clone();
                        let item_id_dec = item_id.clone();

                        rsx! {
                            div {
                                key: "{item.id}",
                                class: if curr_qty > 0 {
                                    "bg-slate-950 border-2 border-indigo-500 rounded-xl p-4 flex flex-col justify-between shadow-lg transition-all"
                                } else {
                                    "bg-slate-950/60 border border-slate-800 rounded-xl p-4 flex flex-col justify-between hover:border-slate-700 transition-all"
                                },

                                div { class: "space-y-1 mb-3",
                                    div { class: "flex items-center justify-between",
                                        h4 { class: "text-sm font-extrabold text-white m-0", "{item.name}" }
                                        span { class: "text-xs font-mono font-bold text-indigo-400 bg-indigo-950 px-2 py-0.5 rounded border border-indigo-800",
                                            "{item.default_volume_m3} m³"
                                        }
                                    }
                                    div { class: "text-[11px] text-slate-400 font-mono",
                                        "Vikt: {item.default_weight_kg} kg"
                                    }
                                    if let Some(ref notes) = item.default_handling_notes {
                                        div { class: "text-[10px] text-amber-400/90 italic mt-1",
                                            "⚠️ {notes}"
                                        }
                                    }
                                }

                                // Interactive Quantity Counter (Finger / Glove Touch targets)
                                div { class: "flex items-center justify-between bg-slate-900 border border-slate-800 rounded-lg p-1.5",
                                    button {
                                        class: "h-9 w-9 bg-slate-800 hover:bg-slate-700 active:scale-95 text-slate-200 font-black text-lg rounded-md flex items-center justify-center transition-all cursor-pointer disabled:opacity-30",
                                        disabled: curr_qty == 0,
                                        onclick: move |_| {
                                            let mut map = item_quantities.read().clone();
                                            let q = map.entry(item_id_dec.clone()).or_insert(0);
                                            if *q > 0 {
                                                *q -= 1;
                                            }
                                            item_quantities.set(map);
                                        },
                                        "-"
                                    }

                                    span { class: "text-base font-black text-white font-mono px-3",
                                        "{curr_qty}"
                                    }

                                    button {
                                        class: "h-9 w-9 bg-indigo-600 hover:bg-indigo-500 active:scale-95 text-white font-black text-lg rounded-md flex items-center justify-center transition-all cursor-pointer shadow-md",
                                        onclick: move |_| {
                                            let mut map = item_quantities.read().clone();
                                            let q = map.entry(item_id_inc.clone()).or_insert(0);
                                            *q += 1;
                                            item_quantities.set(map);
                                        },
                                        "+"
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
