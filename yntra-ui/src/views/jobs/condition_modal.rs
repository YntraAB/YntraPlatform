use crate::components;
use dioxus::prelude::*;
use yntra_core::{
    record_damage_inspection, get_job_damage_inspections, acknowledge_damage_inspection_by_client,
    DamageInspection, MoveInventoryItem,
};

#[derive(Props, Clone, PartialEq)]
pub struct InventoryConditionModalProps {
    pub job_id: String,
    pub active_user_id: String,
    pub inventories: Vec<MoveInventoryItem>,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn InventoryConditionModal(props: InventoryConditionModalProps) -> Element {
    let toast = dioxus_primitives::toast::use_toast();
    let job_id = props.job_id.clone();
    let active_user_id = props.active_user_id.clone();
    let inventories = props.inventories.clone();

    let mut db_trigger = use_signal(|| 0u32);
    let db_trig_val = *db_trigger.read();

    let jid = job_id.clone();
    let uid = active_user_id.clone();
    let inspections_res = use_resource(move || {
        let _ = db_trig_val;
        let j_id = jid.clone();
        let u_id = uid.clone();
        async move {
            get_job_damage_inspections(u_id, j_id).await.unwrap_or_default()
        }
    });

    let inspections_list = inspections_res.read().clone().unwrap_or_default();

    // Condition Code Selection
    let mut selected_item_id = use_signal(|| {
        inventories.first().map(|i| i.id.clone()).unwrap_or_default()
    });
    let mut selected_condition_code = use_signal(|| "SC".to_string());
    let mut selected_severity = use_signal(|| "minor".to_string());
    let mut notes_input = use_signal(|| String::new());

    let mut client_signer_name = use_signal(|| String::new());
    let mut show_waiver_sign = use_signal(|| false);

    let condition_codes = vec![
        ("SC", "Scratched", "Surface scratch or scuff"),
        ("CH", "Chipped", "Chipped edge or finish"),
        ("D", "Dented", "Surface dent or depression"),
        ("MCU", "Missing/Chipped/Uph.", "Upholstery tear / missing piece"),
        ("MAR", "Marred", "Rub mark or stain"),
        ("BR", "Broken", "Structural fracture or break"),
        ("F", "Faded", "Color fading or sun bleach"),
        ("Z", "Cracked", "Glass or wood stress crack"),
    ];

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 overflow-y-auto animate-in fade-in duration-200",
            onclick: move |_| props.on_close.call(()),
            div {
                class: "bg-card border border-border rounded-xl shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col overflow-hidden text-foreground",
                onclick: |e| e.stop_propagation(),

                // Modal Header
                div { class: "p-6 border-b border-border flex items-center justify-between bg-muted/30",
                    div { class: "flex items-center gap-3",
                        div { class: "p-2.5 rounded-lg bg-amber-500/10 text-amber-500 border border-amber-500/20",
                            components::LucideIcon { name: "clipboard-check", size: "22" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground flex items-center gap-2",
                                "Pre-Move Condition Inspection & Damage Waiver"
                                span { class: "text-xs font-semibold px-2 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20",
                                    "Standard Mover Notation"
                                }
                            }
                            p { class: "text-xs text-muted-foreground mt-0.5",
                                "Itemized pre-existing damage logging & signed client liability waiver"
                            }
                        }
                    }
                    button {
                        class: "p-2 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground transition-colors border-0 bg-transparent cursor-pointer",
                        onclick: move |_| props.on_close.call(()),
                        components::LucideIcon { name: "x", size: "18" }
                    }
                }

                // Body
                div { class: "p-6 overflow-y-auto space-y-6 flex-1",
                    // New Condition Log Card
                    div { class: "p-5 rounded-xl bg-muted/30 border border-border space-y-4",
                        h3 { class: "text-sm font-bold text-foreground flex items-center gap-2 m-0",
                            components::LucideIcon { name: "plus-circle", size: "16", class: "text-primary" }
                            "Log Itemized Pre-Existing Condition"
                        }

                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            // Select Inventory Item
                            div { class: "space-y-1.5",
                                label { class: "text-xs font-semibold text-muted-foreground", "Target Furniture Item" }
                                select {
                                    class: "w-full px-3 py-2 text-xs rounded-md bg-background border border-border text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                    value: "{selected_item_id}",
                                    onchange: move |e| selected_item_id.set(e.value()),
                                    {inventories.iter().map(|item| {
                                        let item_id = item.id.clone();
                                        let room = item.room_name.as_deref().unwrap_or("General");
                                        rsx! {
                                            option { key: "{item_id}", value: "{item_id}",
                                                "[{room}] {item.item_name} (Qty: {item.quantity})"
                                            }
                                        }
                                    })}
                                }
                            }

                            // Select Severity
                            div { class: "space-y-1.5",
                                label { class: "text-xs font-semibold text-muted-foreground", "Damage Severity" }
                                select {
                                    class: "w-full px-3 py-2 text-xs rounded-md bg-background border border-border text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                    value: "{selected_severity}",
                                    onchange: move |e| selected_severity.set(e.value()),
                                    option { value: "minor", "Minor (Light wear/cosmetic)" }
                                    option { value: "moderate", "Moderate (Noticeable flaw)" }
                                    option { value: "severe", "Severe (Pre-existing structural damage)" }
                                }
                            }
                        }

                        // Industry Condition Notation Tags
                        div { class: "space-y-2",
                            label { class: "text-xs font-semibold text-muted-foreground", "Select Standard Mover Condition Code" }
                            div { class: "grid grid-cols-2 sm:grid-cols-4 gap-2",
                                {condition_codes.into_iter().map(|(code, label, desc)| {
                                    let is_selected = *selected_condition_code.read() == code;
                                    let btn_class = if is_selected {
                                        "p-2.5 rounded-lg border-2 border-primary bg-primary/10 text-foreground font-bold text-xs flex flex-col items-start text-left cursor-pointer transition-all shadow-sm"
                                    } else {
                                        "p-2.5 rounded-lg border border-border bg-background hover:bg-muted text-muted-foreground hover:text-foreground text-xs flex flex-col items-start text-left cursor-pointer transition-all"
                                    };
                                    rsx! {
                                        button {
                                            key: "{code}",
                                            class: "{btn_class}",
                                            onclick: move |_| selected_condition_code.set(code.to_string()),
                                            div { class: "flex items-center gap-1.5 font-mono text-xs font-extrabold text-primary",
                                                span { class: "px-1.5 py-0.5 rounded bg-primary/20 text-primary", "{code}" }
                                                "{label}"
                                            }
                                            div { class: "text-[10px] text-muted-foreground mt-1 line-clamp-1", "{desc}" }
                                        }
                                    }
                                })}
                            }
                        }

                        // Additional Location / Annotation Notes
                        div { class: "space-y-1.5",
                            label { class: "text-xs font-semibold text-muted-foreground", "Specific Notes & Location on Item" }
                            input {
                                r#type: "text",
                                class: "w-full px-3 py-2 text-xs rounded-md bg-background border border-border text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                placeholder: "e.g., Deep scratch on back right leg, scuff on top surface edge",
                                value: "{notes_input}",
                                oninput: move |e| notes_input.set(e.value())
                            }
                        }

                        button {
                            class: "w-full py-2.5 px-4 rounded-lg bg-amber-600 hover:bg-amber-500 text-white font-semibold text-xs transition-colors border-0 cursor-pointer flex items-center justify-center gap-2 shadow-sm",
                            onclick: {
                                let active_uid = active_user_id.clone();
                                let j_id = job_id.clone();
                                let invs = inventories.clone();
                                let mut d_trig = db_trigger;
                                move |_| {
                                    let item_id_val = selected_item_id.read().clone();
                                    let target_item = invs.iter().find(|i| i.id == item_id_val);
                                    let item_name_str = target_item.map(|i| i.item_name.clone()).unwrap_or_else(|| "General Item".to_string());
                                    let code_val = selected_condition_code.read().clone();
                                    let sev_val = selected_severity.read().clone();
                                    let notes_val = notes_input.read().clone();

                                    let uid = active_uid.clone();
                                    let jid = j_id.clone();
                                    spawn(async move {
                                        let res = record_damage_inspection(
                                            uid, jid, Some(item_id_val), item_name_str, code_val, sev_val, Some(notes_val), None
                                        ).await;
                                        match res {
                                            Ok(_) => {
                                                toast.success("Condition inspection code logged".to_string(), dioxus_primitives::toast::ToastOptions::new());
                                                notes_input.set(String::new());
                                                let current = *d_trig.read();
                                                d_trig.set(current + 1);
                                            }
                                            Err(e) => {
                                                toast.error(format!("Failed to record condition: {}", e), dioxus_primitives::toast::ToastOptions::new());
                                            }
                                        }
                                    });
                                }
                            },
                            components::LucideIcon { name: "plus", size: "16" }
                            "Log Pre-Existing Condition Record"
                        }
                    }

                    // Existing Logged Condition Manifest Table
                    div { class: "space-y-3",
                        div { class: "flex items-center justify-between",
                            h3 { class: "text-sm font-bold text-foreground flex items-center gap-2 m-0",
                                components::LucideIcon { name: "list-checks", size: "16" }
                                "Pre-Move Item Condition Manifest ({inspections_list.len()})"
                            }
                            if !inspections_list.is_empty() {
                                button {
                                    class: "px-3 py-1.5 rounded-md bg-emerald-600/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-600/20 text-xs font-semibold cursor-pointer transition-colors flex items-center gap-1.5",
                                    onclick: move |_| show_waiver_sign.set(true),
                                    components::LucideIcon { name: "file-signature", size: "14" }
                                    "Sign Client Pre-Move Damage Waiver"
                                }
                            }
                        }

                        if inspections_list.is_empty() {
                            div { class: "p-6 rounded-xl border border-dashed border-border text-center text-xs text-muted-foreground bg-muted/10",
                                "No pre-existing item damage condition codes logged yet for this move."
                            }
                        } else {
                            div { class: "border border-border rounded-xl overflow-hidden bg-background",
                                table { class: "w-full text-left border-collapse text-xs",
                                    thead { class: "bg-muted/40 text-muted-foreground uppercase text-[10px] font-bold border-b border-border",
                                        tr {
                                            th { class: "p-3", "Item Name" }
                                            th { class: "p-3", "Code" }
                                            th { class: "p-3", "Severity" }
                                            th { class: "p-3", "Notes / Details" }
                                            th { class: "p-3", "Waiver Status" }
                                        }
                                    }
                                    tbody { class: "divide-y divide-border/50",
                                        {inspections_list.iter().map(|insp| {
                                            let insp_id = insp.id.clone();
                                            let ack = insp.client_acknowledged;
                                            rsx! {
                                                tr { key: "{insp_id}", class: "hover:bg-muted/20 transition-colors",
                                                    td { class: "p-3 font-semibold text-foreground", "{insp.item_name}" }
                                                    td { class: "p-3 font-mono font-bold text-amber-400",
                                                        span { class: "px-2 py-0.5 rounded bg-amber-500/10 border border-amber-500/20",
                                                            "{insp.damage_type}"
                                                        }
                                                    }
                                                    td { class: "p-3 capitalize text-muted-foreground", "{insp.severity}" }
                                                    td { class: "p-3 text-muted-foreground max-w-xs truncate",
                                                        "{insp.annotations.as_deref().unwrap_or(\"None\")}"
                                                    }
                                                    td { class: "p-3 font-medium",
                                                        if ack {
                                                            span { class: "text-emerald-400 flex items-center gap-1 text-[11px]",
                                                                components::LucideIcon { name: "check-circle", size: "13" }
                                                                "Client Signed Waiver"
                                                            }
                                                        } else {
                                                            span { class: "text-amber-400 text-[11px]", "Pending Waiver Sign" }
                                                        }
                                                    }
                                                }
                                            }
                                        })}
                                    }
                                }
                            }
                        }
                    }

                    // Waiver Signatures Form
                    if *show_waiver_sign.read() {
                        div { class: "p-5 rounded-xl border border-emerald-500/30 bg-emerald-500/5 space-y-4 animate-in fade-in duration-150",
                            div { class: "flex items-center justify-between",
                                h4 { class: "text-sm font-bold text-foreground flex items-center gap-2",
                                    components::LucideIcon { name: "shield-check", size: "18", class: "text-emerald-400" }
                                    "Client Pre-Move Condition Acknowledgment & Damage Waiver"
                                }
                                button {
                                    class: "text-xs text-muted-foreground hover:text-foreground border-0 bg-transparent cursor-pointer",
                                    onclick: move |_| show_waiver_sign.set(false),
                                    "Cancel"
                                }
                            }
                            p { class: "text-xs text-muted-foreground leading-relaxed",
                                "I acknowledge that the itemized pre-existing condition codes noted above reflect the condition of my items prior to loading by the carrier."
                            }
                            div { class: "space-y-1.5",
                                label { class: "text-xs font-semibold text-muted-foreground", "Client Full Name" }
                                input {
                                    r#type: "text",
                                    class: "w-full px-3 py-2 text-xs rounded-md bg-background border border-border text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                    placeholder: "e.g., Customer / Authorized Representative",
                                    value: "{client_signer_name}",
                                    oninput: move |e| client_signer_name.set(e.value())
                                }
                            }
                            button {
                                class: "w-full py-2.5 px-4 rounded-md bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs transition-colors border-0 cursor-pointer flex items-center justify-center gap-2 shadow-sm",
                                onclick: {
                                    let active_uid = active_user_id.clone();
                                    let list_clone = inspections_list.clone();
                                    let signer_val = client_signer_name.read().clone();
                                    let mut d_trig = db_trigger;
                                    move |_| {
                                        if signer_val.trim().is_empty() {
                                            toast.error("Client name is required for waiver sign-off".to_string(), dioxus_primitives::toast::ToastOptions::new());
                                            return;
                                        }
                                        let uid = active_uid.clone();
                                        let items_to_ack: Vec<String> = list_clone.iter().map(|i| i.id.clone()).collect();
                                        let mock_sig_svg = format!("<svg>mock_waiver_sig_{}</svg>", signer_val);
                                        spawn(async move {
                                            for insp_id in items_to_ack {
                                                let _ = acknowledge_damage_inspection_by_client(uid.clone(), insp_id, Some(mock_sig_svg.clone())).await;
                                            }
                                            toast.success("Client pre-move damage waiver signed successfully".to_string(), dioxus_primitives::toast::ToastOptions::new());
                                            show_waiver_sign.set(false);
                                            let current = *d_trig.read();
                                            d_trig.set(current + 1);
                                        });
                                    }
                                },
                                components::LucideIcon { name: "check-circle-2", size: "16" }
                                "Sign & Seal Damage Waiver"
                            }
                        }
                    }
                }

                // Modal Footer
                div { class: "p-4 border-t border-border bg-muted/20 flex items-center justify-end gap-3",
                    button {
                        class: "px-4 py-2 rounded-md bg-muted hover:bg-accent text-foreground text-xs font-medium transition-colors border border-border cursor-pointer",
                        onclick: move |_| props.on_close.call(()),
                        "Close"
                    }
                }
            }
        }
    }
}
