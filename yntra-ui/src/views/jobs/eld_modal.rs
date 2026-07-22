use dioxus::prelude::*;

#[component]
pub fn EldDotComplianceModal(
    vehicle_id: String,
    on_close_handler: EventHandler<()>,
) -> Element {
    let mut active_tab = use_signal(|| "HOS".to_string());
    
    // ELD HOS State
    let driver_id_input = use_signal(|| "drv-101".to_string());
    let driver_name_input = use_signal(|| "Erik Driver".to_string());
    let mut hos_status_input = use_signal(|| "ON_DUTY_DRIVING".to_string());
    let mut driving_hours_input = use_signal(|| "6.5".to_string());
    let mut on_duty_hours_input = use_signal(|| "8.5".to_string());
    let mut cycle_hours_input = use_signal(|| "42.0".to_string());

    // DVIR Inspection State
    let mut inspection_type_input = use_signal(|| "PRE_TRIP".to_string());
    let mut brakes_ok = use_signal(|| true);
    let mut tires_ok = use_signal(|| true);
    let mut lights_ok = use_signal(|| true);
    let mut steering_ok = use_signal(|| true);
    let mut coupling_ok = use_signal(|| true);
    let mut defect_details = use_signal(String::new);

    // GVWR State
    let mut cargo_weight_input = use_signal(|| "5800".to_string());
    let mut tare_weight_input = use_signal(|| "4500".to_string());
    let mut gvwr_input = use_signal(|| "12000".to_string());

    // IFTA Log State
    let mut from_jurisdiction_input = use_signal(|| "SE-AB".to_string());
    let mut to_jurisdiction_input = use_signal(|| "SE-VG".to_string());
    let mut odometer_input = use_signal(|| "142500".to_string());
    let mut fuel_liters_input = use_signal(|| "160.0".to_string());

    let mut feedback_msg = use_signal(|| Option::<String>::None);
    let mut db_trigger = use_signal(|| 0u32);

    let v_id = vehicle_id.clone();
    let summary_res = use_resource(move || {
        let vid = v_id.clone();
        let _trig = *db_trigger.read();
        async move {
            yntra_core::get_vehicle_dot_compliance_summary("user-staff".to_string(), vid).await.ok()
        }
    });

    let summary_opt = summary_res.read().clone().flatten();

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-4 font-sans text-slate-100 animate-in fade-in duration-200",

            div {
                class: "bg-slate-900 border border-slate-700/80 rounded-2xl w-full max-w-4xl max-h-[90vh] flex flex-col shadow-2xl overflow-hidden",

                // Modal Header
                div {
                    class: "px-6 py-4 bg-slate-950 border-b border-slate-800 flex items-center justify-between",
                    div {
                        class: "flex items-center gap-3",
                        span { class: "text-3xl", "🚛" }
                        div {
                            h2 { class: "text-xl font-black text-white tracking-wide uppercase", "ELD / HOS & DOT COMPLIANCE ENGINE" }
                            p { class: "text-xs text-slate-400 font-mono", "FORDON ID: {vehicle_id} • REGELEFTERLEVNAD FORDON & FÖRARE" }
                        }
                    }
                    button {
                        class: "h-9 w-9 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-lg flex items-center justify-center font-bold text-lg transition-all",
                        onclick: move |_| on_close_handler.call(()),
                        "✕"
                    }
                }

                // Compliance Overview Banner
                div {
                    class: "px-6 py-3 bg-slate-950/60 border-b border-slate-800 flex items-center justify-between text-xs font-mono",
                    if let Some(ref s) = summary_opt {
                        div { class: "flex items-center gap-4",
                            div {
                                class: if s.is_dot_compliant { "px-2.5 py-1 bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 rounded-md font-bold" } else { "px-2.5 py-1 bg-rose-500/20 text-rose-300 border border-rose-500/40 rounded-md font-bold" },
                                if s.is_dot_compliant { "✓ DOT GODKÄND & REGELEFTERLEVD" } else { "⚠️ OMEDELBAR ATÅGÄRD KRÄVS" }
                            }
                            div { class: "text-slate-300", "AKTIV HOS: {s.active_hos_status}" }
                            div { class: "text-slate-300", "ÖVERTRÄDELSER: {s.hos_violation_count}" }
                            div { class: "text-slate-300", "SENASTE DVIR: {s.latest_dvir_status}" }
                            div { class: "text-slate-300", "IFTA-GRÄNSKORSNINGAR: {s.total_ifta_jurisdictions_logged}" }
                        }
                    } else {
                        div { class: "text-slate-400 animate-pulse", "Hämtar DOT efterlevnadsstatus..." }
                    }
                }

                // Tab Selector
                div {
                    class: "flex border-b border-slate-800 bg-slate-950/40 px-6 pt-2 gap-2 text-sm font-semibold",
                    button {
                        class: if *active_tab.read() == "HOS" { "px-4 py-2.5 bg-indigo-600 text-white rounded-t-xl font-bold border-t border-x border-indigo-500 shadow-lg" } else { "px-4 py-2.5 text-slate-400 hover:text-slate-200 rounded-t-xl hover:bg-slate-800/40 transition-all" },
                        onclick: move |_| active_tab.set("HOS".to_string()),
                        "⏱️ ELD HOS Logg"
                    }
                    button {
                        class: if *active_tab.read() == "DVIR" { "px-4 py-2.5 bg-indigo-600 text-white rounded-t-xl font-bold border-t border-x border-indigo-500 shadow-lg" } else { "px-4 py-2.5 text-slate-400 hover:text-slate-200 rounded-t-xl hover:bg-slate-800/40 transition-all" },
                        onclick: move |_| active_tab.set("DVIR".to_string()),
                        "📋 DVIR Inspektion"
                    }
                    button {
                        class: if *active_tab.read() == "GVWR" { "px-4 py-2.5 bg-indigo-600 text-white rounded-t-xl font-bold border-t border-x border-indigo-500 shadow-lg" } else { "px-4 py-2.5 text-slate-400 hover:text-slate-200 rounded-t-xl hover:bg-slate-800/40 transition-all" },
                        onclick: move |_| active_tab.set("GVWR".to_string()),
                        "⚖️ GVWR Viktkontroll"
                    }
                    button {
                        class: if *active_tab.read() == "IFTA" { "px-4 py-2.5 bg-indigo-600 text-white rounded-t-xl font-bold border-t border-x border-indigo-500 shadow-lg" } else { "px-4 py-2.5 text-slate-400 hover:text-slate-200 rounded-t-xl hover:bg-slate-800/40 transition-all" },
                        onclick: move |_| active_tab.set("IFTA".to_string()),
                        "🌐 IFTA Drivmedel & Gräns"
                    }
                }

                // Feedback Banner
                if let Some(ref msg) = *feedback_msg.read() {
                    div {
                        class: "mx-6 mt-4 p-3 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl text-xs font-mono font-bold flex items-center justify-between",
                        span { "✓ {msg}" }
                        button { class: "text-emerald-400 font-bold ml-2", onclick: move |_| feedback_msg.set(None), "✕" }
                    }
                }

                // Tab Content Body
                div {
                    class: "flex-1 overflow-y-auto p-6 space-y-6",

                    if *active_tab.read() == "HOS" {
                        // TAB 1: ELD HOS LOG
                        div {
                            class: "space-y-4",
                            h3 { class: "text-base font-bold text-white uppercase tracking-wider", "ELD Kör- & Arbetstidsregistrering (Hours of Service)" }

                            div {
                                class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Förare ID" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{driver_id_input}",
                                        readonly: true
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Förare Namn" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{driver_name_input}",
                                        readonly: true
                                    }
                                }
                            }

                            div {
                                class: "grid grid-cols-2 sm:grid-cols-4 gap-3",
                                button {
                                    class: if *hos_status_input.read() == "OFF_DUTY" { "py-3 bg-slate-700 border-2 border-indigo-400 text-white rounded-xl font-bold text-xs uppercase shadow-md" } else { "py-3 bg-slate-950 border border-slate-800 text-slate-400 hover:text-white rounded-xl text-xs uppercase" },
                                    onclick: move |_| hos_status_input.set("OFF_DUTY".to_string()),
                                    "☕ Ej i Tjänst (Off Duty)"
                                }
                                button {
                                    class: if *hos_status_input.read() == "SLEEPER_BERTH" { "py-3 bg-indigo-700 border-2 border-indigo-400 text-white rounded-xl font-bold text-xs uppercase shadow-md" } else { "py-3 bg-slate-950 border border-slate-800 text-slate-400 hover:text-white rounded-xl text-xs uppercase" },
                                    onclick: move |_| hos_status_input.set("SLEEPER_BERTH".to_string()),
                                    "🛌 Sovhytt (Sleeper)"
                                }
                                button {
                                    class: if *hos_status_input.read() == "ON_DUTY_DRIVING" { "py-3 bg-emerald-600 border-2 border-white text-white rounded-xl font-extrabold text-xs uppercase shadow-md" } else { "py-3 bg-slate-950 border border-slate-800 text-slate-400 hover:text-white rounded-xl text-xs uppercase" },
                                    onclick: move |_| hos_status_input.set("ON_DUTY_DRIVING".to_string()),
                                    "🚚 Körning (Driving)"
                                }
                                button {
                                    class: if *hos_status_input.read() == "ON_DUTY_NOT_DRIVING" { "py-3 bg-amber-600 border-2 border-white text-white rounded-xl font-extrabold text-xs uppercase shadow-md" } else { "py-3 bg-slate-950 border border-slate-800 text-slate-400 hover:text-white rounded-xl text-xs uppercase" },
                                    onclick: move |_| hos_status_input.set("ON_DUTY_NOT_DRIVING".to_string()),
                                    "📦 Lastning / Arbete"
                                }
                            }

                            div {
                                class: "grid grid-cols-1 md:grid-cols-3 gap-4 pt-2",
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Körtid Idag (max 11h)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{driving_hours_input}",
                                        oninput: move |e| driving_hours_input.set(e.value())
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Arbetstid Idag (max 14h fönster)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{on_duty_hours_input}",
                                        oninput: move |e| on_duty_hours_input.set(e.value())
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "7-Dagars Cykeltid (max 70h)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{cycle_hours_input}",
                                        oninput: move |e| cycle_hours_input.set(e.value())
                                    }
                                }
                            }

                            button {
                                class: "w-full py-3 bg-indigo-600 hover:bg-indigo-500 active:scale-[0.99] text-white font-bold text-sm rounded-xl shadow-lg transition-all border border-indigo-400 flex items-center justify-center gap-2",
                                onclick: move |_| {
                                    let v_id = vehicle_id.clone();
                                    let drv_id = driver_id_input.read().clone();
                                    let drv_name = driver_name_input.read().clone();
                                    let st = hos_status_input.read().clone();
                                    let dh = driving_hours_input.read().parse::<f64>().unwrap_or(0.0);
                                    let od = on_duty_hours_input.read().parse::<f64>().unwrap_or(0.0);
                                    let cy = cycle_hours_input.read().parse::<f64>().unwrap_or(0.0);

                                    spawn(async move {
                                        if let Ok(rec) = yntra_core::log_eld_hos_status(
                                            "user-staff".to_string(),
                                            drv_id,
                                            drv_name,
                                            v_id,
                                            st,
                                            dh,
                                            od,
                                            cy,
                                        ).await {
                                            let mut msg = format!("HOS Status registrerad: {}", rec.status);
                                            if rec.violation_flag {
                                                msg = format!("⚠️ ÖVERTRÄDELSE DETEKTERAD: {}", rec.violation_reason.unwrap_or_default());
                                            }
                                            feedback_msg.set(Some(msg));
                                            let curr = *db_trigger.read();
                                            db_trigger.set(curr + 1);
                                        }
                                    });
                                },
                                "💾 SPARA & SIGNERA ELD LOGGPOST"
                            }
                        }
                    } else if *active_tab.read() == "DVIR" {
                        // TAB 2: DVIR INSPECTION
                        div {
                            class: "space-y-4",
                            h3 { class: "text-base font-bold text-white uppercase tracking-wider", "DVIR (Driver Vehicle Inspection Report) Säkerhetskontroll" }

                            div {
                                class: "flex items-center gap-4 bg-slate-950 p-3 border border-slate-800 rounded-xl",
                                button {
                                    class: if *inspection_type_input.read() == "PRE_TRIP" { "px-4 py-2 bg-indigo-600 text-white font-bold text-xs rounded-lg uppercase" } else { "px-4 py-2 bg-slate-900 text-slate-400 text-xs rounded-lg uppercase" },
                                    onclick: move |_| inspection_type_input.set("PRE_TRIP".to_string()),
                                    "🔍 Före Körning (Pre-Trip)"
                                }
                                button {
                                    class: if *inspection_type_input.read() == "POST_TRIP" { "px-4 py-2 bg-indigo-600 text-white font-bold text-xs rounded-lg uppercase" } else { "px-4 py-2 bg-slate-900 text-slate-400 text-xs rounded-lg uppercase" },
                                    onclick: move |_| inspection_type_input.set("POST_TRIP".to_string()),
                                    "🏁 Efter Körning (Post-Trip)"
                                }
                            }

                            div {
                                class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                                button {
                                    class: if *brakes_ok.read() { "p-3 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl font-bold text-xs flex items-center justify-between" } else { "p-3 bg-rose-500/20 border border-rose-500/40 text-rose-300 rounded-xl font-bold text-xs flex items-center justify-between" },
                                    onclick: move |_| {
                                        let curr = *brakes_ok.read();
                                        brakes_ok.set(!curr);
                                    },
                                    span { "🛑 Bromssystem & Tryckluft" }
                                    span { if *brakes_ok.read() { "✓ GODKÄND" } else { "❌ DEFEKT" } }
                                }

                                button {
                                    class: if *tires_ok.read() { "p-3 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl font-bold text-xs flex items-center justify-between" } else { "p-3 bg-rose-500/20 border border-rose-500/40 text-rose-300 rounded-xl font-bold text-xs flex items-center justify-between" },
                                    onclick: move |_| {
                                        let curr = *tires_ok.read();
                                        tires_ok.set(!curr);
                                    },
                                    span { "🛞 Däck, Fälgar & Lufttryck" }
                                    span { if *tires_ok.read() { "✓ GODKÄND" } else { "❌ DEFEKT" } }
                                }

                                button {
                                    class: if *lights_ok.read() { "p-3 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl font-bold text-xs flex items-center justify-between" } else { "p-3 bg-rose-500/20 border border-rose-500/40 text-rose-300 rounded-xl font-bold text-xs flex items-center justify-between" },
                                    onclick: move |_| {
                                        let curr = *lights_ok.read();
                                        lights_ok.set(!curr);
                                    },
                                    span { "💡 Belysning, Blinkers & Reflexer" }
                                    span { if *lights_ok.read() { "✓ GODKÄND" } else { "❌ DEFEKT" } }
                                }

                                button {
                                    class: if *steering_ok.read() { "p-3 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl font-bold text-xs flex items-center justify-between" } else { "p-3 bg-rose-500/20 border border-rose-500/40 text-rose-300 rounded-xl font-bold text-xs flex items-center justify-between" },
                                    onclick: move |_| {
                                        let curr = *steering_ok.read();
                                        steering_ok.set(!curr);
                                    },
                                    span { "🎯 Styrsystem & Framvagn" }
                                    span { if *steering_ok.read() { "✓ GODKÄND" } else { "❌ DEFEKT" } }
                                }

                                button {
                                    class: if *coupling_ok.read() { "p-3 bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 rounded-xl font-bold text-xs flex items-center justify-between" } else { "p-3 bg-rose-500/20 border border-rose-500/40 text-rose-300 rounded-xl font-bold text-xs flex items-center justify-between" },
                                    onclick: move |_| {
                                        let curr = *coupling_ok.read();
                                        coupling_ok.set(!curr);
                                    },
                                    span { "🔗 Kopplingsanordningar & Släp" }
                                    span { if *coupling_ok.read() { "✓ GODKÄND" } else { "❌ DEFEKT" } }
                                }
                            }

                            div {
                                label { class: "block text-xs text-slate-400 mb-1 font-medium", "Anmärkningar & Defektbeskrivning" }
                                textarea {
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg p-3 text-sm text-white font-mono focus:border-indigo-500 outline-none h-20",
                                    placeholder: "Beskriv ev. brister, slitna däck eller luftläckage...",
                                    value: "{defect_details}",
                                    oninput: move |e| defect_details.set(e.value())
                                }
                            }

                            button {
                                class: "w-full py-3 bg-indigo-600 hover:bg-indigo-500 active:scale-[0.99] text-white font-bold text-sm rounded-xl shadow-lg transition-all border border-indigo-400 flex items-center justify-center gap-2",
                                onclick: move |_| {
                                    let v_id = vehicle_id.clone();
                                    let drv_id = driver_id_input.read().clone();
                                    let ins_type = inspection_type_input.read().clone();
                                    let b_ok = *brakes_ok.read();
                                    let t_ok = *tires_ok.read();
                                    let l_ok = *lights_ok.read();
                                    let s_ok = *steering_ok.read();
                                    let c_ok = *coupling_ok.read();
                                    let det = if defect_details.read().trim().is_empty() { None } else { Some(defect_details.read().clone()) };

                                    spawn(async move {
                                        if let Ok(rep) = yntra_core::submit_dvir_inspection(
                                            "user-staff".to_string(),
                                            v_id,
                                            drv_id,
                                            ins_type,
                                            b_ok,
                                            t_ok,
                                            l_ok,
                                            s_ok,
                                            c_ok,
                                            det,
                                        ).await {
                                            feedback_msg.set(Some(format!("DVIR Insänd: Status {}", rep.safety_status)));
                                            let curr = *db_trigger.read();
                                            db_trigger.set(curr + 1);
                                        }
                                    });
                                },
                                "📋 SPARA & SKICKA DVIR RAPPORT"
                            }
                        }
                    } else if *active_tab.read() == "GVWR" {
                        // TAB 3: GVWR OVERLOAD CHECK
                        div {
                            class: "space-y-4",
                            h3 { class: "text-base font-bold text-white uppercase tracking-wider", "Gross Vehicle Weight Rating (GVWR) Viktberäkning" }

                            div {
                                class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Lastvikt / Gods (kg)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{cargo_weight_input}",
                                        oninput: move |e| cargo_weight_input.set(e.value())
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Tjänstevikt / Fordon (kg)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{tare_weight_input}",
                                        oninput: move |e| tare_weight_input.set(e.value())
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Tillåten Totalvikt / GVWR (kg)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{gvwr_input}",
                                        oninput: move |e| gvwr_input.set(e.value())
                                    }
                                }
                            }

                            button {
                                class: "w-full py-3 bg-indigo-600 hover:bg-indigo-500 active:scale-[0.99] text-white font-bold text-sm rounded-xl shadow-lg transition-all border border-indigo-400 flex items-center justify-center gap-2",
                                onclick: move |_| {
                                    let v_id = vehicle_id.clone();
                                    let c_w = cargo_weight_input.read().parse::<f64>().unwrap_or(0.0);
                                    let t_w = tare_weight_input.read().parse::<f64>().unwrap_or(0.0);
                                    let gv = gvwr_input.read().parse::<f64>().unwrap_or(0.0);

                                    spawn(async move {
                                        if let Ok(w) = yntra_core::check_gvwr_overload_status(
                                            "user-staff".to_string(),
                                            v_id,
                                            c_w,
                                            t_w,
                                            gv,
                                        ).await {
                                            let msg = if w.is_overloaded {
                                                format!("⚠️ ÖVERLAST DETEKTERAD! Total: {:.0}kg (Över med {:.0}kg) - Severitet: {}", w.total_actual_gross_weight_kg, w.overload_margin_kg, w.warning_severity)
                                            } else {
                                                format!("✓ VIKT GODKÄND: Total {:.0}kg / Max {:.0}kg", w.total_actual_gross_weight_kg, w.gvwr_kg)
                                            };
                                            feedback_msg.set(Some(msg));
                                        }
                                    });
                                },
                                "⚖️ BERÄKNA & KONTROLLERA ÖVERLAST"
                            }
                        }
                    } else if *active_tab.read() == "IFTA" {
                        // TAB 4: IFTA STATE LINE FUEL LOG
                        div {
                            class: "space-y-4",
                            h3 { class: "text-base font-bold text-white uppercase tracking-wider", "International Fuel Tax Agreement (IFTA) Gräns- & Drivmedelslogg" }

                            div {
                                class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Från Region / Län" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{from_jurisdiction_input}",
                                        oninput: move |e| from_jurisdiction_input.set(e.value())
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Till Region / Län" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{to_jurisdiction_input}",
                                        oninput: move |e| to_jurisdiction_input.set(e.value())
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Mätarställning Vid Gräns (km)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{odometer_input}",
                                        oninput: move |e| odometer_input.set(e.value())
                                    }
                                }
                                div {
                                    label { class: "block text-xs text-slate-400 mb-1 font-medium", "Tankat Drivmedel (Liter)" }
                                    input {
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white font-mono focus:border-indigo-500 outline-none",
                                        value: "{fuel_liters_input}",
                                        oninput: move |e| fuel_liters_input.set(e.value())
                                    }
                                }
                            }

                            button {
                                class: "w-full py-3 bg-indigo-600 hover:bg-indigo-500 active:scale-[0.99] text-white font-bold text-sm rounded-xl shadow-lg transition-all border border-indigo-400 flex items-center justify-center gap-2",
                                onclick: move |_| {
                                    let v_id = vehicle_id.clone();
                                    let drv_id = driver_id_input.read().clone();
                                    let fj = from_jurisdiction_input.read().clone();
                                    let tj = to_jurisdiction_input.read().clone();
                                    let odo = odometer_input.read().parse::<f64>().unwrap_or(0.0);
                                    let fuel = fuel_liters_input.read().parse::<f64>().unwrap_or(0.0);

                                    spawn(async move {
                                        if let Ok(ifta) = yntra_core::log_ifta_jurisdiction_crossing(
                                            "user-staff".to_string(),
                                            v_id,
                                            drv_id,
                                            fj,
                                            tj,
                                            odo,
                                            fuel,
                                        ).await {
                                            feedback_msg.set(Some(format!("IFTA Post Registrerad: {} -> {} (Mätare: {:.0}km, Tankat: {:.1}L)", ifta.from_jurisdiction, ifta.to_jurisdiction, ifta.odometer_km, ifta.fuel_purchased_liters)));
                                            let curr = *db_trigger.read();
                                            db_trigger.set(curr + 1);
                                        }
                                    });
                                },
                                "🌐 REGISTERRA IFTA GRÄNSKORSNING & TANKNING"
                            }
                        }
                    }
                }
            }
        }
    }
}
