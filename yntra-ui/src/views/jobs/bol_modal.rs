use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{
    BillOfLading, generate_bill_of_lading, get_bill_of_lading, sign_bill_of_lading_phase,
};

#[derive(Props, Clone, PartialEq)]
pub struct BillOfLadingModalProps {
    pub job_id: String,
    pub active_user_id: String,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn BillOfLadingModal(props: BillOfLadingModalProps) -> Element {
    let toast = dioxus_primitives::toast::use_toast();
    let job_id = props.job_id.clone();
    let active_user_id = props.active_user_id.clone();

    let mut db_trigger = use_signal(|| 0u32);
    let db_trig_val = *db_trigger.read();

    let jid = job_id.clone();
    let bol_res = use_resource(move || {
        let _ = db_trig_val;
        let j_id = jid.clone();
        async move { get_bill_of_lading(j_id).await.unwrap_or(None) }
    });

    let bol_data = bol_res.read().clone().flatten();

    // Form inputs for generating or updating BOL
    let mut carrier_name_input = use_signal(|| "Yntra Logistics & Freight Inc.".to_string());
    let mut valuation_option = use_signal(|| "released_value_060".to_string());
    let mut declared_value_input = use_signal(|| 50000.0f64);
    let mut deductible_input = use_signal(|| 250.0f64);

    let mut signer_name_input = use_signal(|| "".to_string());
    let mut show_signature_pad = use_signal(|| false);
    let mut active_sig_phase = use_signal(|| "origin".to_string());

    let is_generating = use_signal(|| false);

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 overflow-y-auto animate-in fade-in duration-200",
            onclick: move |_| props.on_close.call(()),
            div {
                class: "bg-card border border-border rounded-xl shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col overflow-hidden text-foreground",
                onclick: |e| e.stop_propagation(),

                // Header
                div { class: "p-6 border-b border-border flex items-center justify-between bg-muted/30",
                    div { class: "flex items-center gap-3",
                        div { class: "p-2.5 rounded-lg bg-primary/10 text-primary border border-primary/20",
                            components::LucideIcon { name: "file-text", size: "22" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground flex items-center gap-2",
                                "Bill of Lading (BOL)"
                                span { class: "text-xs font-semibold px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                    "FMCSA Carmack Compliant"
                                }
                            }
                            p { class: "text-xs text-muted-foreground mt-0.5",
                                "Official Contract of Carriage & Liability Valuation Disclosure (49 U.S.C. § 14706)"
                            }
                        }
                    }
                    button {
                        class: "p-2 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground transition-colors border-0 bg-transparent cursor-pointer",
                        onclick: move |_| props.on_close.call(()),
                        components::LucideIcon { name: "x", size: "18" }
                    }
                }

                // Body Content
                div { class: "p-6 overflow-y-auto space-y-6 flex-1",
                    if let Some(ref bol) = bol_data {
                        // BOL Metadata Cards
                        div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                            div { class: "p-4 rounded-lg bg-muted/40 border border-border space-y-1.5",
                                div { class: "text-xs text-muted-foreground font-medium uppercase tracking-wider", "Document Control" }
                                div { class: "text-sm font-mono font-bold text-primary", "{bol.bol_number}" }
                                div { class: "text-xs text-muted-foreground",
                                    "USDOT #: "
                                    span { class: "font-mono font-medium text-foreground",
                                        "{bol.carrier_dot_number.as_deref().unwrap_or(\"Pending / Registered\")}"
                                    }
                                }
                            }
                            div { class: "p-4 rounded-lg bg-muted/40 border border-border space-y-1.5",
                                div { class: "text-xs text-muted-foreground font-medium uppercase tracking-wider", "Shipper & Freight" }
                                div { class: "text-sm font-semibold text-foreground truncate", "{bol.shipper_name}" }
                                div { class: "text-xs text-muted-foreground",
                                    "Est. Total Weight: "
                                    span { class: "font-semibold text-emerald-400", "{bol.total_estimated_weight_lbs:.0} lbs" }
                                }
                            }
                            div { class: "p-4 rounded-lg bg-muted/40 border border-border space-y-1.5",
                                div { class: "text-xs text-muted-foreground font-medium uppercase tracking-wider", "Tamper Verification" }
                                div { class: "text-xs font-mono truncate text-muted-foreground title={bol.document_tamper_hash.clone()}",
                                    "{bol.document_tamper_hash}"
                                }
                                div { class: "flex items-center gap-1.5 text-[11px] text-emerald-400 font-medium",
                                    components::LucideIcon { name: "shield-check", size: "14" }
                                    "SHA256 Encrypted Audit"
                                }
                            }
                        }

                        // Origin & Destination Addresses
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 p-4 rounded-lg bg-background border border-border",
                            div { class: "space-y-1",
                                div { class: "text-xs font-semibold text-muted-foreground uppercase flex items-center gap-1.5",
                                    components::LucideIcon { name: "map-pin", size: "14" }
                                    "Origin Pickup Address"
                                }
                                div { class: "text-sm font-medium text-foreground", "{bol.origin_address}" }
                            }
                            div { class: "space-y-1",
                                div { class: "text-xs font-semibold text-muted-foreground uppercase flex items-center gap-1.5",
                                    components::LucideIcon { name: "navigation", size: "14" }
                                    "Destination Delivery Address"
                                }
                                div { class: "text-sm font-medium text-foreground", "{bol.destination_address}" }
                            }
                        }

                        // Valuation Protection Terms
                        div { class: "p-5 rounded-xl border border-primary/20 bg-primary/5 space-y-3",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-2",
                                    components::LucideIcon { name: "shield-alert", size: "18" }
                                    h3 { class: "text-sm font-bold text-foreground", "Carrier Liability & Valuation Option Selected" }
                                }
                                span { class: "text-xs font-semibold px-2.5 py-1 rounded-full bg-primary/20 text-primary border border-primary/30",
                                    if bol.valuation_option == "released_value_060" {
                                        "Released Value ($0.60/lb)"
                                    } else {
                                        "Full Value Protection"
                                    }
                                }
                            }
                            div { class: "grid grid-cols-1 md:grid-cols-3 gap-3 text-xs",
                                div { class: "bg-background/80 p-3 rounded-md border border-border",
                                    div { class: "text-muted-foreground", "Declared Protection Amount" }
                                    div { class: "text-sm font-bold text-foreground mt-0.5", "${bol.valuation_declared_amount:.2}" }
                                }
                                div { class: "bg-background/80 p-3 rounded-md border border-border",
                                    div { class: "text-muted-foreground", "Deductible" }
                                    div { class: "text-sm font-bold text-foreground mt-0.5", "${bol.valuation_deductible:.2}" }
                                }
                                div { class: "bg-background/80 p-3 rounded-md border border-border",
                                    div { class: "text-muted-foreground", "Valuation Premium Surcharge" }
                                    div { class: "text-sm font-bold text-emerald-400 mt-0.5", "${bol.valuation_premium:.2}" }
                                }
                            }
                        }

                        // FMCSA Carmack Legal Disclosures
                        div { class: "p-4 rounded-lg bg-muted/20 border border-border space-y-2",
                            h4 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground", "STB Carmack Amendment Terms & Disclosures" }
                            p { class: "text-xs text-muted-foreground leading-relaxed font-mono bg-background p-3 rounded border border-border/50 max-h-28 overflow-y-auto",
                                "{bol.legal_terms}"
                            }
                        }

                        // Signature Status & Dual Phase Controls
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            // Origin Signature Box
                            div { class: "p-4 rounded-lg border border-border bg-card space-y-3",
                                div { class: "flex items-center justify-between",
                                    h4 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5",
                                        components::LucideIcon { name: "file-signature", size: "14" }
                                        "Phase 1: Pickup Signature"
                                    }
                                    if bol.origin_signature_hash.is_some() {
                                        span { class: "text-xs font-medium text-emerald-400 flex items-center gap-1",
                                            components::LucideIcon { name: "check-circle-2", size: "14" }
                                            "Signed"
                                        }
                                    } else {
                                        span { class: "text-xs font-medium text-amber-400", "Pending Sign" }
                                    }
                                }
                                if let Some(ref h) = bol.origin_signature_hash {
                                    div { class: "text-xs font-mono text-muted-foreground truncate bg-muted/30 p-2 rounded",
                                        "Hash: {h}"
                                    }
                                } else {
                                    button {
                                        class: "w-full py-2 px-3 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-colors border-0 cursor-pointer flex items-center justify-center gap-2",
                                        onclick: move |_| {
                                            active_sig_phase.set("origin".to_string());
                                            show_signature_pad.set(true);
                                        },
                                        components::LucideIcon { name: "pen-tool", size: "14" }
                                        "Sign Pickup (Origin)"
                                    }
                                }
                            }

                            // Destination Signature Box
                            div { class: "p-4 rounded-lg border border-border bg-card space-y-3",
                                div { class: "flex items-center justify-between",
                                    h4 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5",
                                        components::LucideIcon { name: "file-check", size: "14" }
                                        "Phase 2: Delivery Signature"
                                    }
                                    if bol.destination_signature_hash.is_some() {
                                        span { class: "text-xs font-medium text-emerald-400 flex items-center gap-1",
                                            components::LucideIcon { name: "check-circle-2", size: "14" }
                                            "Signed & Delivered"
                                        }
                                    } else {
                                        span { class: "text-xs font-medium text-amber-400", "Pending Delivery" }
                                    }
                                }
                                if let Some(ref h) = bol.destination_signature_hash {
                                    div { class: "text-xs font-mono text-muted-foreground truncate bg-muted/30 p-2 rounded",
                                        "Hash: {h}"
                                    }
                                } else {
                                    button {
                                        class: "w-full py-2 px-3 rounded-md bg-emerald-600 text-white text-xs font-medium hover:bg-emerald-500 transition-colors border-0 cursor-pointer flex items-center justify-center gap-2",
                                        onclick: move |_| {
                                            active_sig_phase.set("destination".to_string());
                                            show_signature_pad.set(true);
                                        },
                                        components::LucideIcon { name: "pen-tool", size: "14" }
                                        "Sign Delivery (Destination)"
                                    }
                                }
                            }
                        }

                        // Signature Pad Modal Input
                        if *show_signature_pad.read() {
                            div { class: "p-4 rounded-xl border border-primary/30 bg-muted/50 space-y-3 animate-in fade-in duration-150",
                                div { class: "flex items-center justify-between",
                                    h4 { class: "text-sm font-bold text-foreground", "Sign Bill of Lading Phase ({active_sig_phase.read().to_uppercase()})" }
                                    button {
                                        class: "text-xs text-muted-foreground hover:text-foreground border-0 bg-transparent cursor-pointer",
                                        onclick: move |_| show_signature_pad.set(false),
                                        "Cancel"
                                    }
                                }
                                div { class: "space-y-2",
                                    label { class: "text-xs text-muted-foreground font-medium", "Signer Full Name" }
                                    input {
                                        r#type: "text",
                                        class: "w-full px-3 py-2 text-xs rounded-md bg-background border border-border text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                        placeholder: "e.g., John Smith (Customer / Authorized Rep)",
                                        value: "{signer_name_input}",
                                        oninput: move |e| signer_name_input.set(e.value())
                                    }
                                }
                                button {
                                    class: "w-full py-2 px-4 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-colors border-0 cursor-pointer flex items-center justify-center gap-2",
                                    onclick: {
                                        let j_id = job_id.clone();
                                        let active_uid = active_user_id.clone();
                                        let phase_val = active_sig_phase.read().clone();
                                        let signer_val = signer_name_input.read().clone();
                                        let mut d_trig = db_trigger;
                                        move |_| {
                                            if signer_val.trim().is_empty() {
                                                toast.error("Signer name is required".to_string(), dioxus_primitives::toast::ToastOptions::new());
                                                return;
                                            }
                                            let uid = active_uid.clone();
                                            let jid = j_id.clone();
                                            let ph = phase_val.clone();
                                            let s_name = signer_val.clone();
                                            let mock_sig_b64 = format!("data:image/png;base64,mock_sig_{}", j_id);
                                            spawn(async move {
                                                let res = sign_bill_of_lading_phase(uid, jid, ph, s_name, mock_sig_b64).await;
                                                match res {
                                                    Ok(_) => {
                                                        toast.success("Bill of Lading signature recorded".to_string(), dioxus_primitives::toast::ToastOptions::new());
                                                        show_signature_pad.set(false);
                                                        let current = *d_trig.read();
                                                        d_trig.set(current + 1);
                                                    }
                                                    Err(e) => {
                                                        toast.error(format!("Signature failed: {}", e), dioxus_primitives::toast::ToastOptions::new());
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    components::LucideIcon { name: "check-circle", size: "14" }
                                    "Confirm Digital Signature"
                                }
                            }
                        }
                    } else {
                        // Generate BOL Setup Screen
                        div { class: "p-6 rounded-xl border border-dashed border-border text-center space-y-5 bg-muted/10",
                            div { class: "w-12 h-12 rounded-full bg-primary/10 text-primary mx-auto flex items-center justify-center border border-primary/20",
                                components::LucideIcon { name: "file-plus", size: "24" }
                            }
                            div { class: "space-y-1 max-w-md mx-auto",
                                h3 { class: "text-base font-bold text-foreground", "No Bill of Lading Generated Yet" }
                                p { class: "text-xs text-muted-foreground",
                                    "Generate an official FMCSA Carmack compliant Bill of Lading contract for this job, selecting carrier liability and valuation protection terms."
                                }
                            }

                            div { class: "max-w-md mx-auto text-left space-y-4 bg-background p-4 rounded-lg border border-border",
                                div { class: "space-y-1.5",
                                    label { class: "text-xs font-semibold text-muted-foreground", "Carrier Legal Name" }
                                    input {
                                        r#type: "text",
                                        class: "w-full px-3 py-2 text-xs rounded-md bg-muted/30 border border-border text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                        value: "{carrier_name_input}",
                                        oninput: move |e| carrier_name_input.set(e.value())
                                    }
                                }

                                div { class: "space-y-1.5",
                                    label { class: "text-xs font-semibold text-muted-foreground", "Valuation Coverage Option" }
                                    select {
                                        class: "w-full px-3 py-2 text-xs rounded-md bg-muted/30 border border-border text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                        value: "{valuation_option}",
                                        onchange: move |e| valuation_option.set(e.value()),
                                        option { value: "released_value_060", "Released Value Protection ($0.60/lb per article - Standard Default)" }
                                        option { value: "full_value_protection", "Full Value Protection (Full Replacement Value)" }
                                    }
                                }

                                if *valuation_option.read() == "full_value_protection" {
                                    div { class: "grid grid-cols-2 gap-3 animate-in fade-in duration-150",
                                        div { class: "space-y-1",
                                            label { class: "text-xs text-muted-foreground", "Declared Total Value ($)" }
                                            input {
                                                r#type: "number",
                                                class: "w-full px-3 py-2 text-xs rounded-md bg-muted/30 border border-border text-foreground",
                                                value: "{declared_value_input}",
                                                oninput: move |e| {
                                                    if let Ok(v) = e.value().parse::<f64>() {
                                                        declared_value_input.set(v);
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "space-y-1",
                                            label { class: "text-xs text-muted-foreground", "Deductible ($)" }
                                            input {
                                                r#type: "number",
                                                class: "w-full px-3 py-2 text-xs rounded-md bg-muted/30 border border-border text-foreground",
                                                value: "{deductible_input}",
                                                oninput: move |e| {
                                                    if let Ok(v) = e.value().parse::<f64>() {
                                                        deductible_input.set(v);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            button {
                                class: "px-6 py-2.5 rounded-lg bg-primary text-primary-foreground font-semibold text-xs hover:bg-primary/90 transition-colors border-0 cursor-pointer inline-flex items-center gap-2 shadow-lg shadow-primary/20",
                                onclick: {
                                    let j_id = job_id.clone();
                                    let active_uid = active_user_id.clone();
                                    let carrier = carrier_name_input.read().clone();
                                    let val_opt = valuation_option.read().clone();
                                    let dec_val = *declared_value_input.read();
                                    let ded_val = *deductible_input.read();
                                    let mut d_trig = db_trigger;
                                    move |_| {
                                        let uid = active_uid.clone();
                                        let jid = j_id.clone();
                                        let c_name = carrier.clone();
                                        let v_opt = val_opt.clone();
                                        spawn(async move {
                                            let res = generate_bill_of_lading(uid, jid, c_name, v_opt, dec_val, ded_val).await;
                                            match res {
                                                Ok(_) => {
                                                    toast.success("Bill of Lading generated successfully".to_string(), dioxus_primitives::toast::ToastOptions::new());
                                                    let current = *d_trig.read();
                                                    d_trig.set(current + 1);
                                                }
                                                Err(e) => {
                                                    toast.error(format!("Failed to generate BOL: {}", e), dioxus_primitives::toast::ToastOptions::new());
                                                }
                                            }
                                        });
                                    }
                                },
                                components::LucideIcon { name: "file-check-2", size: "16" }
                                "Generate Bill of Lading Document"
                            }
                        }
                    }
                }

                // Modal Footer
                div { class: "p-4 border-t border-border bg-muted/20 flex items-center justify-between",
                    div { class: "text-xs text-muted-foreground flex items-center gap-1.5",
                        components::LucideIcon { name: "lock", size: "14" }
                        "Cryptographically Sealed & Encrypted Contract"
                    }
                    div { class: "flex items-center gap-3",
                        if let Some(ref bol) = bol_data {
                            button {
                                class: "px-4 py-2 rounded-md bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold transition-colors border-0 cursor-pointer flex items-center gap-2 shadow-sm",
                                onclick: {
                                    let active_uid = active_user_id.clone();
                                    let j_id = job_id.clone();
                                    let bol_num = bol.bol_number.clone();
                                    move |_| {
                                        let uid = active_uid.clone();
                                        let jid = j_id.clone();
                                        let bnum = bol_num.clone();
                                        spawn(async move {
                                            if let Ok(html) = yntra_core::generate_printable_bol_html(uid, jid).await {
                                                super::printable_exporter::trigger_print_or_pdf_download(&toast, &html, &format!("{}.html", bnum));
                                            }
                                        });
                                    }
                                },
                                components::LucideIcon { name: "printer", size: "14" }
                                "Print / Save PDF BOL"
                            }
                        }
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
}

#[allow(dead_code)]
fn trigger_download_file(toast: &dioxus_primitives::toast::Toasts, content: &str, file_name: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        toast.info(
            "Exporting Bill of Lading...".to_string(),
            dioxus_primitives::toast::ToastOptions::new(),
        );
        let base64_str = base64_encode_str(content.as_bytes());
        let js_code = format!(
            r#"
            (function() {{
                const base64 = "{}";
                const filename = "{}";
                const binString = atob(base64);
                const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                const blob = new Blob([bytes], {{ type: "text/plain;charset=utf-8" }});
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = filename;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);
            }})();
            "#,
            base64_str, file_name
        );
        let _ = dioxus::document::eval(&js_code);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let file_path = rfd::FileDialog::new().set_file_name(file_name).save_file();
        if let Some(path) = file_path {
            if std::fs::write(&path, content).is_ok() {
                toast.success(
                    format!("Saved Bill of Lading to {}", path.display()),
                    dioxus_primitives::toast::ToastOptions::new(),
                );
            } else {
                toast.error(
                    "Failed to save Bill of Lading document".to_string(),
                    dioxus_primitives::toast::ToastOptions::new(),
                );
            }
        }
    }
}

#[allow(dead_code)]
fn base64_encode_str(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as u32
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        encoded.push(CHARS[((triple >> 18) & 63) as usize] as char);
        encoded.push(CHARS[((triple >> 12) & 63) as usize] as char);
        if i + 1 < data.len() {
            encoded.push(CHARS[((triple >> 6) & 63) as usize] as char);
        } else {
            encoded.push('=');
        }
        if i + 2 < data.len() {
            encoded.push(CHARS[(triple & 63) as usize] as char);
        } else {
            encoded.push('=');
        }
        i += 3;
    }
    encoded
}
