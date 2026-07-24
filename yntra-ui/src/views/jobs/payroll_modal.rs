use dioxus::prelude::*;
use crate::components;

#[component]
pub fn CrewPayrollModal(
    job_id: String,
    active_user_id: String,
    on_close: EventHandler<()>,
) -> Element {
    let toast = dioxus_primitives::toast::use_toast();
    let mut db_trigger = use_signal(|| 0u32);

    let mut tip_input = use_signal(|| "600".to_string());
    let mut driving_hours_input = use_signal(|| "2.5".to_string());
    let mut loading_hours_input = use_signal(|| "5.5".to_string());
    let mut is_overnight_per_diem = use_signal(|| false);

    let mut selected_crew_id = use_signal(|| String::new());

    let mut is_distributing = use_signal(|| false);
    let mut is_calculating = use_signal(|| false);
    let mut payroll_breakdown = use_signal(|| Option::<yntra_core::MoverPayrollBreakdown>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let uid = active_user_id.clone();
    let jid = job_id.clone();
    let trig_val = *db_trigger.read();
    let crew_res = use_resource(move || {
        let _ = trig_val;
        let u = uid.clone();
        let j = jid.clone();
        async move {
            yntra_core::get_job_crew(u, j).await.unwrap_or_default()
        }
    });

    let uid_b = active_user_id.clone();
    let jid_b = job_id.clone();
    let tip_res = use_resource(move || {
        let _ = trig_val;
        let u = uid_b.clone();
        let j = jid_b.clone();
        async move {
            yntra_core::get_job_tip_distribution(u, j).await.ok().flatten()
        }
    });

    let handle_distribute_tip = {
        let active_uid = active_user_id.clone();
        let j_id = job_id.clone();
        move |_| {
            let uid = active_uid.clone();
            let jid = j_id.clone();
            let amt = tip_input.read().parse::<f64>().unwrap_or(0.0);

            if amt <= 0.0 {
                error_msg.set(Some("Vänligen ange ett giltigt dricksbelopp".to_string()));
                return;
            }

            is_distributing.set(true);
            error_msg.set(None);

            spawn(async move {
                let res = yntra_core::distribute_job_customer_tip(uid, jid, amt).await;
                is_distributing.set(false);
                match res {
                    Ok(_) => {
                        toast.success(
                            "Dricks fördelad jämnt över hela laget!".to_string(),
                            dioxus_primitives::toast::ToastOptions::new(),
                        );
                        let current = *db_trigger.read();
                        db_trigger.set(current + 1);
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Dricksfördelning misslyckades: {}", e)));
                    }
                }
            });
        }
    };

    let handle_calculate_payroll = {
        let active_uid = active_user_id.clone();
        let j_id = job_id.clone();
        move |_| {
            let uid = active_uid.clone();
            let jid = j_id.clone();
            let target_user = selected_crew_id.read().clone();
            let drv_h = driving_hours_input.read().parse::<f64>().unwrap_or(0.0);
            let load_h = loading_hours_input.read().parse::<f64>().unwrap_or(0.0);
            let per_diem = *is_overnight_per_diem.read();

            if target_user.is_empty() {
                error_msg.set(Some("Välj en medarbetare för att beräkna lön".to_string()));
                return;
            }

            is_calculating.set(true);
            error_msg.set(None);

            spawn(async move {
                let res = yntra_core::calculate_mover_job_payroll_split(
                    uid,
                    jid,
                    target_user,
                    drv_h,
                    load_h,
                    per_diem,
                ).await;

                is_calculating.set(false);
                match res {
                    Ok(bd) => {
                        payroll_breakdown.set(Some(bd));
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Löneberäkning misslyckades: {}", e)));
                    }
                }
            });
        }
    };

    let crew_list = crew_res.read().clone().unwrap_or_default();
    let tip_dist = tip_res.read().clone().flatten();

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4 animate-in fade-in duration-200",
            onclick: move |_| on_close.call(()),

            div {
                class: "bg-background border border-border rounded-2xl shadow-2xl w-full max-w-xl overflow-hidden flex flex-col max-h-[90vh]",
                onclick: move |e| e.stop_propagation(),

                // Header
                div { class: "p-4 border-b border-border bg-muted/40 flex justify-between items-center",
                    div { class: "flex items-center gap-2",
                        div { class: "p-2 rounded-lg bg-emerald-500/10 text-emerald-500 border border-emerald-500/20",
                            components::LucideIcon { name: "coins", size: "18" }
                        }
                        div {
                            h3 { class: "text-sm font-extrabold text-foreground m-0", "Dricksfördelning & Förarlön (Payroll Split)" }
                            p { class: "text-[10px] text-muted-foreground m-0", "Fördela kunddricks & beräkna kör- vs bärtimmar, övertid och traktamente" }
                        }
                    }
                    button {
                        class: "p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer border-0 bg-transparent",
                        onclick: move |_| on_close.call(()),
                        components::LucideIcon { name: "x", size: "16" }
                    }
                }

                // Body
                div { class: "p-5 space-y-4 overflow-y-auto flex-1 text-xs",

                    // Tip Allocation Card
                    div { class: "bg-emerald-500/10 border border-emerald-500/20 rounded-xl p-4 space-y-3 text-left",
                        div { class: "flex items-center justify-between",
                            div { class: "flex items-center gap-2",
                                components::LucideIcon { name: "heart-handshake", size: "16", class: "text-emerald-400" }
                                span { class: "font-bold text-xs text-foreground", "Kunddricks (Tips)" }
                            }
                            if let Some(ref t) = tip_dist {
                                span { class: "text-[10px] font-bold bg-emerald-500/20 text-emerald-300 px-2.5 py-0.5 rounded-full border border-emerald-500/30",
                                    "{t.tip_per_member_sek:.0} SEK / person ({t.crew_count} st lag)"
                                }
                            }
                        }

                        div { class: "flex gap-2 items-center pt-1",
                            input {
                                r#type: "number",
                                step: "50",
                                placeholder: "Totalt dricksbelopp i SEK",
                                class: "flex-1 px-3 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs font-bold",
                                value: "{tip_input}",
                                oninput: move |e| tip_input.set(e.value())
                            }
                            button {
                                class: "px-4 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-xs cursor-pointer transition-all border-0 flex items-center gap-1.5 shadow",
                                disabled: *is_distributing.read(),
                                onclick: handle_distribute_tip,
                                if *is_distributing.read() {
                                    components::LucideIcon { name: "loader-2", class: "animate-spin h-3.5 w-3.5" }
                                    "Fördelar..."
                                } else {
                                    components::LucideIcon { name: "users", size: "13" }
                                    "Fördela på laget"
                                }
                            }
                        }
                    }

                    // Payroll Calculator Card
                    div { class: "bg-muted/30 border border-border/60 rounded-xl p-4 space-y-3 text-left",
                        h4 { class: "text-[11px] font-bold text-foreground m-0 flex items-center gap-1.5",
                            components::LucideIcon { name: "calculator", size: "13" }
                            "Beräkna timlön & tillägg per flyttarbetare"
                        }

                        div { class: "grid grid-cols-2 gap-2",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Medarbetare i laget" }
                                select {
                                    class: "px-2.5 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs",
                                    value: "{selected_crew_id}",
                                    onchange: move |e| selected_crew_id.set(e.value()),
                                    option { value: "", "Välj flyttare..." }
                                    for c in crew_list.iter() {
                                        option { key: "{c.id}", value: "{c.id}", "{c.id} ({c.role})" }
                                    }
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Traktamente (Övernattning)" }
                                label { class: "flex items-center gap-2 pt-1.5 cursor-pointer text-xs font-semibold text-foreground",
                                    input {
                                        r#type: "checkbox",
                                        checked: *is_overnight_per_diem.read(),
                                        onchange: move |e| is_overnight_per_diem.set(e.value().parse().unwrap_or(false))
                                    }
                                    "Ja, 290 SEK per diem"
                                }
                            }
                        }

                        div { class: "grid grid-cols-2 gap-2",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Körtid / Transport (h)" }
                                input {
                                    r#type: "number",
                                    step: "0.5",
                                    class: "px-2.5 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs",
                                    value: "{driving_hours_input}",
                                    oninput: move |e| driving_hours_input.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Bär- & Lasttid (h)" }
                                input {
                                    r#type: "number",
                                    step: "0.5",
                                    class: "px-2.5 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs",
                                    value: "{loading_hours_input}",
                                    oninput: move |e| loading_hours_input.set(e.value())
                                }
                            }
                        }

                        button {
                            class: "w-full py-2 rounded-xl bg-primary text-primary-foreground font-bold text-xs cursor-pointer shadow hover:opacity-90 transition-all border-0 flex items-center justify-center gap-2 mt-1",
                            disabled: *is_calculating.read(),
                            onclick: handle_calculate_payroll,
                            if *is_calculating.read() {
                                components::LucideIcon { name: "loader-2", class: "animate-spin h-4 w-4" }
                                "Beräknar..."
                            } else {
                                components::LucideIcon { name: "pie-chart", size: "14" }
                                "Beräkna Lönespecifikation"
                            }
                        }
                    }

                    // Payroll Output Breakdown Display
                    if let Some(ref bd) = *payroll_breakdown.read() {
                        div { class: "bg-background border border-border rounded-xl p-4 space-y-2 text-left shadow-sm animate-in fade-in duration-200",
                            div { class: "flex justify-between items-center border-b border-border/40 pb-2",
                                div { class: "font-black text-xs text-foreground", "Lönespecifikation - Uppdrag" }
                                span { class: "text-[10px] font-bold font-mono bg-muted px-2 py-0.5 rounded", "{bd.role}" }
                            }
                            div { class: "space-y-1.5 text-[11px] pt-1",
                                div { class: "flex justify-between text-muted-foreground",
                                    span { "Körtid ({bd.driving_hours}h @ {bd.driving_rate_sek_per_h:.0} SEK/h):" }
                                    span { class: "font-mono font-bold text-foreground", "{bd.driving_hours * bd.driving_rate_sek_per_h:.2} SEK" }
                                }
                                div { class: "flex justify-between text-muted-foreground",
                                    span { "Bärtid ({bd.loading_hours}h @ {bd.loading_rate_sek_per_h:.0} SEK/h):" }
                                    span { class: "font-mono font-bold text-foreground", "{bd.loading_hours * bd.loading_rate_sek_per_h:.2} SEK" }
                                }
                                if bd.overtime_hours > 0.0 {
                                    div { class: "flex justify-between text-amber-400 font-semibold",
                                        span { "Övertidstillägg ({bd.overtime_hours}h @ 1.5x):" }
                                        span { class: "font-mono font-bold", "{bd.overtime_hours * (bd.loading_rate_sek_per_h * 1.5):.2} SEK" }
                                    }
                                }
                                if bd.per_diem_allowance_sek > 0.0 {
                                    div { class: "flex justify-between text-primary font-semibold",
                                        span { "Traktamente (Per Diem):" }
                                        span { class: "font-mono font-bold", "{bd.per_diem_allowance_sek:.2} SEK" }
                                    }
                                }
                                if bd.tip_allocated_sek > 0.0 {
                                    div { class: "flex justify-between text-emerald-400 font-semibold",
                                        span { "Allokerad Kunddricks:" }
                                        span { class: "font-mono font-bold", "{bd.tip_allocated_sek:.2} SEK" }
                                    }
                                }
                                div { class: "flex justify-between text-xs font-black text-foreground pt-2 border-t border-border/40",
                                    span { "Totalt Brutto Utbetalning:" }
                                    span { class: "text-emerald-400 text-sm font-black font-mono", "{bd.total_gross_payout_sek:.2} SEK" }
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

                // Footer
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
