use dioxus::prelude::*;
use crate::components;

#[component]
pub fn WarehouseSitModal(
    job_id: String,
    active_user_id: String,
    on_close: EventHandler<()>,
) -> Element {
    let toast = dioxus_primitives::toast::use_toast();
    let mut db_trigger = use_signal(|| 0u32);

    let mut vault_number_input = use_signal(|| String::new());
    let mut warehouse_name_input = use_signal(|| "Central Huvudlager #1 (Bredden)".to_string());
    let mut allocated_volume_input = use_signal(|| "12.5".to_string());
    let mut monthly_rate_input = use_signal(|| "1850.0".to_string());
    let mut is_submitting = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let uid = active_user_id.clone();
    let jid = job_id.clone();
    let trig_val = *db_trigger.read();
    let vaults_res = use_resource(move || {
        let _ = trig_val;
        let u = uid.clone();
        let j = jid.clone();
        async move {
            yntra_core::get_job_warehouse_vaults(u, j).await.unwrap_or_default()
        }
    });

    let uid_b = active_user_id.clone();
    let jid_b = job_id.clone();
    let billing_res = use_resource(move || {
        let _ = trig_val;
        let u = uid_b.clone();
        let j = jid_b.clone();
        async move {
            yntra_core::calculate_sit_recurring_billing_summary(u, j).await.ok()
        }
    });

    let handle_allocate_vault = {
        let active_uid = active_user_id.clone();
        let j_id = job_id.clone();
        move |_| {
            let uid = active_uid.clone();
            let jid = j_id.clone();
            let v_num = vault_number_input.read().clone();
            let w_name = warehouse_name_input.read().clone();
            let vol = allocated_volume_input.read().parse::<f64>().unwrap_or(10.0);
            let rate = monthly_rate_input.read().parse::<f64>().unwrap_or(1500.0);
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

            if v_num.trim().is_empty() {
                error_msg.set(Some("Vänligen ange ett box-/magasinnummer (t.ex. V-104)".to_string()));
                return;
            }

            is_submitting.set(true);
            error_msg.set(None);

            spawn(async move {
                let res = yntra_core::assign_job_to_warehouse_vault(
                    uid,
                    jid,
                    v_num,
                    w_name,
                    vol,
                    rate,
                    today,
                    None,
                ).await;

                is_submitting.set(false);
                match res {
                    Ok(_) => {
                        toast.success(
                            "Magasineringsbox tilldelad!".to_string(),
                            dioxus_primitives::toast::ToastOptions::new(),
                        );
                        vault_number_input.set(String::new());
                        let current = *db_trigger.read();
                        db_trigger.set(current + 1);
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Misslyckades att tilldela magasin: {}", e)));
                    }
                }
            });
        }
    };

    let vaults_list = vaults_res.read().clone().unwrap_or_default();
    let sit_billing = billing_res.read().clone().flatten();

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
                        div { class: "p-2 rounded-lg bg-primary/10 text-blue-500 border border-primary/20",
                            components::LucideIcon { name: "warehouse", size: "18" }
                        }
                        div {
                            h3 { class: "text-sm font-extrabold text-foreground m-0", "Magasinering & SIT (Storage-in-Transit)" }
                            p { class: "text-[10px] text-muted-foreground m-0", "Spåra lagerhyllor, m³ volym och löpande månadsdebitering" }
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
                    
                    // SIT Recurring Billing Summary Banner
                    if let Some(ref billing) = sit_billing {
                        div { class: "bg-primary/10 border border-primary/20 rounded-xl p-3.5 space-y-2",
                            div { class: "flex justify-between items-center",
                                div { class: "text-[10px] font-bold text-primary uppercase tracking-wider", "Ackumulerad Magasineringshyra (SIT)" }
                                span { class: "text-[10px] font-bold bg-primary/20 text-primary px-2 py-0.5 rounded border border-primary/30",
                                    "{billing.days_in_storage} dagar i lager"
                                }
                            }
                            div { class: "grid grid-cols-3 gap-2 text-left pt-1 border-t border-primary/20",
                                div {
                                    div { class: "text-[9px] text-muted-foreground", "Totalt Volym" }
                                    div { class: "font-black text-sm text-foreground", "{billing.total_volume_m3:.1} m³" }
                                }
                                div {
                                    div { class: "text-[9px] text-muted-foreground", "Månadshyra" }
                                    div { class: "font-black text-sm text-foreground", "{billing.accumulated_storage_fee_sek:.0} SEK" }
                                }
                                div {
                                    div { class: "text-[9px] text-muted-foreground", "In/Ut-hantering" }
                                    div { class: "font-black text-sm text-foreground", "{billing.handling_in_out_fee_sek:.0} SEK" }
                                }
                            }
                        }
                    }

                    // Assigned Vaults List
                    div { class: "space-y-2",
                        h4 { class: "text-[11px] font-bold text-foreground m-0 flex items-center gap-1.5",
                            components::LucideIcon { name: "box", size: "13" }
                            "Tilldelade Magasineringsboxar ({vaults_list.len()})"
                        }

                        if vaults_list.is_empty() {
                            div { class: "p-4 text-center border border-dashed border-border rounded-xl text-muted-foreground text-[11px]",
                                "Inga magasineringsboxar tilldelade ännu för detta uppdrag."
                            }
                        } else {
                            div { class: "space-y-2",
                                for v in vaults_list.iter() {
                                    div {
                                        key: "{v.id}",
                                        class: "p-3 rounded-xl border border-border/80 bg-muted/20 flex items-center justify-between gap-3",
                                        div { class: "space-y-0.5 text-left",
                                            div { class: "flex items-center gap-2",
                                                span { class: "font-black text-foreground font-mono text-xs bg-background px-2 py-0.5 rounded border border-border/50",
                                                    "{v.vault_number}"
                                                }
                                                span { class: format!(
                                                    "text-[9px] font-bold px-2 py-0.5 rounded-full uppercase {}",
                                                    if v.status == "stored" { "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20" } else { "bg-muted text-muted-foreground" }
                                                ),
                                                    if v.status == "stored" { "I Lager" } else { "Frisläppt" }
                                                }
                                            }
                                            div { class: "text-[10px] text-muted-foreground",
                                                "{v.warehouse_name} • {v.allocated_volume_m3} m³ • {v.monthly_rate_sek} SEK/mån (Inskrivning: {v.move_in_date})"
                                            }
                                        }

                                        if v.status == "stored" {
                                            button {
                                                class: "px-2.5 py-1 text-[10px] font-bold rounded bg-amber-500/10 text-amber-400 border border-amber-500/20 hover:bg-amber-500/20 cursor-pointer flex items-center gap-1 transition-colors",
                                                onclick: {
                                                    let active_uid = active_user_id.clone();
                                                    let vid = v.id.clone();
                                                    move |_| {
                                                        let uid = active_uid.clone();
                                                        let vault_id = vid.clone();
                                                        spawn(async move {
                                                            if let Ok(_) = yntra_core::release_job_from_warehouse_vault(uid, vault_id).await {
                                                                toast.success(
                                                                    "Magasin frisläppt för utleverans!".to_string(),
                                                                    dioxus_primitives::toast::ToastOptions::new(),
                                                                );
                                                                let current = *db_trigger.read();
                                                                db_trigger.set(current + 1);
                                                            }
                                                        });
                                                    }
                                                },
                                                components::LucideIcon { name: "truck", size: "11" }
                                                "Frisläpp för utkörning"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Form: Allocate New Vault
                    div { class: "bg-muted/30 border border-border/60 rounded-xl p-3.5 space-y-3 text-left pt-3",
                        h4 { class: "text-[11px] font-bold text-foreground m-0 flex items-center gap-1.5",
                            components::LucideIcon { name: "plus-circle", size: "13" }
                            "Tilldela Ny Magasineringsplats"
                        }

                        div { class: "grid grid-cols-2 gap-2",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Box / Magasinnummer" }
                                input {
                                    r#type: "text",
                                    placeholder: "T.ex. V-408-B",
                                    class: "px-2.5 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs font-mono",
                                    value: "{vault_number_input}",
                                    oninput: move |e| vault_number_input.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Lageranläggning" }
                                select {
                                    class: "px-2.5 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs",
                                    value: "{warehouse_name_input}",
                                    onchange: move |e| warehouse_name_input.set(e.value()),
                                    option { value: "Central Huvudlager #1 (Bredden)", "Central Huvudlager #1 (Bredden)" }
                                    option { value: "SydDepån #2 (Västberga)", "SydDepån #2 (Västberga)" }
                                    option { value: "Kallförvaring #3 (Arlanda)", "Kallförvaring #3 (Arlanda)" }
                                }
                            }
                        }

                        div { class: "grid grid-cols-2 gap-2",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Allokerad Volym (m³)" }
                                input {
                                    r#type: "number",
                                    step: "0.5",
                                    class: "px-2.5 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs",
                                    value: "{allocated_volume_input}",
                                    oninput: move |e| allocated_volume_input.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-semibold text-muted-foreground", "Månadshyra (SEK/mån)" }
                                input {
                                    r#type: "number",
                                    step: "50",
                                    class: "px-2.5 py-1.5 border border-border rounded-lg bg-background text-foreground text-xs",
                                    value: "{monthly_rate_input}",
                                    oninput: move |e| monthly_rate_input.set(e.value())
                                }
                            }
                        }

                        button {
                            class: "w-full py-2 rounded-xl bg-primary text-primary-foreground font-bold text-xs cursor-pointer shadow hover:opacity-90 transition-all border-0 flex items-center justify-center gap-2 mt-1",
                            disabled: *is_submitting.read(),
                            onclick: handle_allocate_vault,
                            if *is_submitting.read() {
                                components::LucideIcon { name: "loader-2", class: "animate-spin h-4 w-4" }
                                "Registrerar..."
                            } else {
                                components::LucideIcon { name: "check-circle-2", size: "14" }
                                "Spara & Tilldela Magasineringsbox"
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
