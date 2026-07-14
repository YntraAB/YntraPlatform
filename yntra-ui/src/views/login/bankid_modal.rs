use crate::components;
use crate::utils::qr::render_qr_svg;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct BankIdModalProps {
    pub show_bankid_modal: Signal<bool>,
    pub bankid_flow_state: Signal<String>,
    pub bankid_progress: Signal<f32>,
    pub bankid_qr_data: Signal<String>,
    pub bankid_pin: Signal<String>,
    pub active_session_id: Signal<Option<String>>,
    pub active_session_token: Signal<Option<String>>,
    pub provider_val: Signal<String>,
    pub norway_mobile: Signal<String>,
    pub norway_birthdate: Signal<String>,
    pub region: String,
}

impl PartialEq for BankIdModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn BankIdModal(props: BankIdModalProps) -> Element {
    let mut show_bankid_modal = props.show_bankid_modal;
    let mut bankid_flow_state = props.bankid_flow_state;
    let bankid_progress = props.bankid_progress;
    let bankid_qr_data = props.bankid_qr_data;
    let mut bankid_pin = props.bankid_pin;
    let active_session_id = props.active_session_id;
    let active_session_token = props.active_session_token;
    let provider_val = props.provider_val;
    let mut norway_mobile = props.norway_mobile;
    let mut norway_birthdate = props.norway_birthdate;

    rsx! {
        components::Dialog {
            open: *show_bankid_modal.read(),
            title: match provider_val.read().as_str() {
                "se_bankid" => "Mobilt BankID".to_string(),
                "no_bankid" => "BankID på mobil".to_string(),
                "dk_mitid" => "MitID".to_string(),
                _ => "National ID".to_string(),
            },
            onclose: move |_| {
                show_bankid_modal.set(false);
                bankid_flow_state.set("idle".to_string());
            },
            div { class: "flex flex-col items-center text-center gap-5 w-full text-sm",
                style: "max-width: 320px;",

                // -- Sweden BankID / Denmark MitID QR Scanning (Production Flow)
                if *provider_val.read() == "se_bankid" || *provider_val.read() == "dk_mitid" {
                    if *bankid_flow_state.read() == "qr_scan" {
                        h3 { class: "m-0 font-extrabold text-foreground",
                            if *provider_val.read() == "se_bankid" { "Skanna QR-kod" } else { "Scan QR-kode" }
                        }
                        p { class: "text-muted-foreground m-0 text-xs",
                            style: "line-height: 1.4;",
                            if *provider_val.read() == "se_bankid" {
                                "Öppna BankID-appen på din telefon og skanna QR-koden nedan."
                            } else {
                                "Åbn din MitID app og scan koden for at fortsætte."
                            }
                        }

                        div {
                            class: "rounded-xl flex items-center justify-center",
                            style: "width: 160px; height: 160px; border: 4px solid var(--accent); position: relative; background: hsl(0, 0%, 0%); overflow: hidden; box-shadow: 0 0 15px rgba(99,102,241,0.2);",
                            div {
                                class: "w-full",
                                style: "position: absolute; left: 0; height: 3px; background: var(--accent); box-shadow: 0 0 8px var(--accent); animation: scan-laser 2s linear infinite; z-index: 10;",
                            }
                            div {
                                style: "width: 130px; height: 130px; display: flex; align-items: center; justify-content: center;",
                                {render_qr_svg(&bankid_qr_data.read())}
                            }
                        }

                        // Launch local client app (Production custom URI schema launcher)
                        a {
                            class: "yntra-btn w-full mt-2 text-center",
                            style: "text-decoration:none;",
                            href: {
                                let sid = active_session_id.read().clone().unwrap_or_else(|| "session-token".to_string());
                                if *provider_val.read() == "se_bankid" {
                                    format!("bankid:///?autostarttoken={}&redirect=yntra://auth", sid)
                                } else {
                                    format!("mitid:///?autostarttoken={}", sid)
                                }
                            },
                            onclick: move |_| {
                                if let (Some(sid), Some(tok)) = (active_session_id.read().clone(), active_session_token.read().clone()) {
                                    bankid_pin.set("local_app".to_string());
                                    spawn(async move {
                                        let _ = yntra_core::submit_bankid_pin(sid, tok, "local_app".to_string()).await;
                                    });
                                }
                            },
                            if *provider_val.read() == "se_bankid" { "BankID på den här enheten" } else { "MitID på denne enhed" }
                        }

                        // Developer/QA Simulator tool to mock scanning the QR code using a phone
                        if cfg!(debug_assertions) {
                            button {
                                class: "bg-transparent text-[11px] cursor-pointer flex items-center w-full justify-center mt-1",
                                style: "border: 1px dashed rgba(147,51,234,0.4); border-radius: 6px; padding: 0.4rem 0.6rem; color: hsl(270, 95.2%, 75.3%); gap: 0.35rem; transition: background 0.2s;",
                                onclick: move |_| {
                                    if let (Some(sid), Some(tok)) = (active_session_id.read().clone(), active_session_token.read().clone()) {
                                        bankid_pin.set("phone_scan".to_string());
                                        spawn(async move {
                                            let _ = yntra_core::submit_bankid_pin(sid, tok, "phone_scan".to_string()).await;
                                        });
                                    }
                                },
                                "⚡ [Dev Mode] Simulera skanning i telefon"
                            }
                        }
                    }
                }

                // -- Norway BankID Form
                if *provider_val.read() == "no_bankid" {
                    if *bankid_flow_state.read() == "no_prompt" {
                        h3 { class: "m-0 font-extrabold text-foreground", "BankID på mobil" }
                        p { class: "text-muted-foreground m-0 text-xs",
                            "Vennligst oppgi mobilnummer og fødselsdato."
                        }
                        input {
                            class: "yntra-input w-full text-center",
                            placeholder: "Mobilnummer (8 siffer)",
                            value: "{norway_mobile}",
                            oninput: move |e| norway_mobile.set(e.value()),
                        }
                        input {
                            class: "yntra-input w-full text-center",
                            placeholder: "Fødselsdato (DDMMÅÅ)",
                            value: "{norway_birthdate}",
                            oninput: move |e| norway_birthdate.set(e.value()),
                        }
                        button {
                            class: "yntra-btn w-full mt-2",
                            disabled: norway_mobile.read().len() < 8 || norway_birthdate.read().len() < 6,
                            onclick: move |_| {
                                if let (Some(sid), Some(tok)) = (active_session_id.read().clone(), active_session_token.read().clone()) {
                                    let payload = format!("{}|{}", norway_mobile.read(), norway_birthdate.read());
                                    spawn(async move {
                                        let _ = yntra_core::submit_bankid_pin(sid, tok, payload).await;
                                    });
                                }
                            },
                            "Start legitimering"
                        }
                    }
                }

                // -- US / Global Smart Card (Collects PIN on screen because reader is connected locally)
                if *provider_val.read() == "us_global" {
                    if *bankid_flow_state.read() == "us_prompt" {
                        h3 { class: "m-0 font-extrabold text-foreground", "Smart Card Authentication" }
                        p { class: "text-muted-foreground m-0 text-xs",
                            style: "line-height:1.4;",
                            "Please insert your CAC / PIV Smart Card into the reader."
                        }
                        div {
                            class: "rounded-xl flex items-center justify-center bg-white/[0.01]",
                            style: "width: 90px; height: 90px; border: 2px dashed var(--border-color); font-size: 2.5rem;",
                            "💳"
                        }
                        button {
                            class: "yntra-btn w-full mt-2",
                            onclick: move |_| {
                                bankid_flow_state.set("pending_pin".to_string());
                            },
                            "Card Detected - Enter PIN"
                        }
                    } else if *bankid_flow_state.read() == "pending_pin" {
                        h3 { class: "m-0 font-extrabold text-foreground", "Enter Card PIN" }
                        p { class: "text-muted-foreground m-0 text-xs",
                            "Please type your 6-digit PIV certificate PIN."
                        }

                        div { class: "flex gap-3 justify-center",
                            style: "margin: 1rem 0;",
                            for i in 0..6 {
                                div {
                                    style: format!("width: 14px; height: 14px; border-radius: 50%; background: {}; border: {}; transition: all 0.15s;", if bankid_pin.read().len() > i { "var(--accent)" } else { "transparent" }, if bankid_pin.read().len() > i { "1px solid var(--accent)" } else { "1px solid var(--border-color)" })
                                }
                            }
                        }

                        div {
                            class: "grid grid-cols-3 gap-2.5 w-full",
                            style: "max-width: 220px;",
                            for digit in ["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
                                button {
                                    class: "yntra-btn secondary p-0 rounded-full text-lg font-bold",
                                    style: "height: 42px; width: 42px; justify-self: center;",
                                    onclick: move |_| {
                                        let current = bankid_pin.read().clone();
                                        if current.len() < 6 {
                                            bankid_pin.set(format!("{}{}", current, digit));
                                        }
                                    },
                                    "{digit}"
                                }
                            }
                            button {
                                class: "yntra-btn secondary p-0 rounded-full text-[11px]",
                                style: "height: 42px; color: var(--danger); width: 42px; justify-self: center;",
                                onclick: move |_| {
                                    bankid_pin.set(String::new());
                                },
                                "Clear"
                            }
                            button {
                                class: "yntra-btn secondary p-0 rounded-full text-lg font-bold",
                                style: "height: 42px; width: 42px; justify-self: center;",
                                onclick: move |_| {
                                    let current = bankid_pin.read().clone();
                                    if current.len() < 6 {
                                        bankid_pin.set(format!("{}0", current));
                                    }
                                },
                                "0"
                            }
                            button {
                                class: "yntra-btn p-0 rounded-full text-xs text-white",
                                style: "height: 42px; width: 42px; justify-self: center; background:var(--accent); border-color:var(--accent);",
                                disabled: bankid_pin.read().len() < 6,
                                onclick: move |_| {
                                    if let (Some(sid), Some(tok)) = (active_session_id.read().clone(), active_session_token.read().clone()) {
                                        let pin_str = bankid_pin.read().clone();
                                        spawn(async move {
                                            let _ = yntra_core::submit_bankid_pin(sid, tok, pin_str).await;
                                        });
                                    }
                                },
                                "OK"
                            }
                        }
                    }
                }

                // -- Shared verifying & success states (Clean production descriptions)
                if *bankid_flow_state.read() == "verifying" {
                    h3 { class: "m-0 font-extrabold text-foreground",
                        if *provider_val.read() == "us_global" { "Verifying Smart Card" } else { "Väntar på godkännande" }
                    }

                    p { class: "text-muted-foreground m-0 text-xs",
                        style: "line-height: 1.4;",
                        if *provider_val.read() == "no_bankid" {
                            "BankID-referanse sendt til mobil. Kontroller referanse 'KNF' og godkjenn med sikkerhetskode i appen."
                        } else if *bankid_pin.read() == "local_app" {
                            if *provider_val.read() == "se_bankid" {
                                "Startar BankID-appen på den här enheten. Slutför legitimeringen i appen."
                            } else {
                                "Starter MitID-appen på denne enhed. Godkend i appen."
                            }
                        } else if *bankid_pin.read() == "phone_scan" {
                            if *provider_val.read() == "dk_mitid" {
                                "Åbn MitID appen på din mobil og godkend anmodningen."
                            } else {
                                "Öppna BankID-appen på din telefon och godkänn legitimeringen."
                            }
                        } else if *provider_val.read() == "us_global" {
                            "Verifying card certificates securely..."
                        } else {
                            "Starta BankID-appen på din telefon eller enhet och godkänn legitimeringen."
                        }
                    }

                    div {
                        class: "w-full",
                        style: "height: 6px; background: var(--border-color); border-radius: 3px; overflow: hidden; margin: 1.5rem 0;",
                        div {
                            style: format!("height: 100%; background: var(--accent); border-radius: 3px; transition: width 0.1s; width: {}%;", bankid_progress.read())
                        }
                    }
                    span { class: "text-xs text-primary font-bold",
                        style: "font-family: monospace;", "{bankid_progress.read():.0}%" }
                } else if *bankid_flow_state.read() == "success" {
                    div {
                        class: "rounded-full flex items-center justify-center",
                        style: "width: 64px; height: 64px; background: rgba(16,185,129,0.1); border: 2px solid var(--success); margin: 1rem 0; box-shadow: 0 0 15px rgba(16,185,129,0.25);",
                        components::LucideIcon { name: "reporting", class: "h-8 w-8 text-success", }
                    }
                    h3 { class: "m-0 font-extrabold",
                        style: "color:var(--success);",
                        if *provider_val.read() == "us_global" { "Authenticated" } else { "Legitimering lyckades" }
                    }
                    p { class: "text-muted-foreground m-0 text-xs",
                        if *provider_val.read() == "us_global" { "Access granted. Welcome!" } else { "Du är nu inloggad i portalen." }
                    }
                }
            }
        }
    }
}
