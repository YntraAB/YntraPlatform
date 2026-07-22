use dioxus::prelude::*;
use crate::components;
use crate::locales::t;

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetItem {
    pub name: String,
    pub volume: f64,
    pub quantity: i64,
}

#[component]
pub fn PublicBookingWidget(workspace_id: String, locale: Option<String>) -> Element {
    let ws_id = workspace_id.clone();
    let loc = locale.unwrap_or_else(|| "sv".to_string());
    
    let mut step = use_signal(|| 1);
    
    // Contact Info
    let mut name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut phone = use_signal(String::new);
    
    // Address Info
    let mut origin = use_signal(String::new);
    let mut destination = use_signal(String::new);
    
    // Inventory Items
    let mut selected_items = use_signal(|| Vec::<WidgetItem>::new());
    let mut custom_item_name = use_signal(String::new);
    let mut custom_item_vol = use_signal(|| 0.5);

    // Preset list of standard items with typical volume in m3
    let presets = vec![
        ("Soffa (3-sits)", 1.5),
        ("Soffa (2-sits)", 1.0),
        ("Fåtölj", 0.6),
        ("Matbord", 0.8),
        ("Stol", 0.15),
        ("Dubbelsäng", 2.0),
        ("Enkelsäng", 1.2),
        ("Garderob", 1.5),
        ("Flyttkartong", 0.1),
    ];

    // Compute live values
    let items_list = selected_items.read().clone();
    let total_volume: f64 = items_list.iter().map(|item| item.volume * item.quantity as f64).sum();
    // SEK estimation: 1500 kr base + 150 kr per m3
    let estimated_price = 1500.0 + (total_volume * 150.0);

    // Submission states
    let mut submitting = use_signal(|| false);
    let mut submit_error = use_signal(|| Option::<String>::None);
    let mut created_job_id = use_signal(|| Option::<String>::None);

    let handle_submit = move |_| {
        let name_val = name.read().clone();
        let email_val = email.read().clone();
        let phone_val = phone.read().clone();
        let origin_val = origin.read().clone();
        let dest_val = destination.read().clone();
        let items_val = selected_items.read().clone();
        let ws_id_c = ws_id.clone();
        
        if name_val.is_empty() || email_val.is_empty() || phone_val.is_empty() || origin_val.is_empty() || dest_val.is_empty() {
            submit_error.set(Some("Vänligen fyll i alla obligatoriska fält.".to_string()));
            return;
        }

        submitting.set(true);
        submit_error.set(None);

        spawn(async move {
            let items_json = serde_json::json!(
                items_val.iter().map(|item| {
                    serde_json::json!({
                        "name": item.name,
                        "quantity": item.quantity,
                        "volume": item.volume
                    })
                }).collect::<Vec<serde_json::Value>>()
            ).to_string();

            let result = yntra_core::submit_public_booking_lead(
                ws_id_c,
                name_val,
                email_val,
                phone_val,
                origin_val,
                dest_val,
                items_json
            ).await;

            submitting.set(false);
            match result {
                Ok(job_id) => {
                    created_job_id.set(Some(job_id));
                    step.set(4); // Success step!
                }
                Err(e) => {
                    submit_error.set(Some(format!("Ett fel inträffade: {}", e)));
                }
            }
        });
    };

    rsx! {
        div { class: "w-full max-w-lg mx-auto bg-background/80 backdrop-blur-md border border-border/60 rounded-2xl shadow-xl overflow-hidden flex flex-col items-stretch",
            style: "min-height: 520px;",
            
            // Header Indicator
            div { class: "bg-gradient-to-r from-primary/10 to-accent/5 p-4 border-b border-border/40 flex justify-between items-center",
                div {
                    h3 { class: "text-sm font-extrabold text-foreground m-0", "Snabb Offert & Bokningsförfrågan" }
                    p { class: "text-[10px] text-muted-foreground mt-0.5 mb-0", "Få ett kostnadsfritt prisförslag direkt online." }
                }
                span { class: "text-[10px] bg-primary/20 text-primary font-bold px-2 py-0.5 rounded-full border border-primary/20",
                    "Steg {step} av 4"
                }
            }

            // Sticky Live Volume & Price Estimate Banner (visible across steps 1..3)
            if *step.read() < 4 {
                div { class: "bg-primary/5 border-b border-border/30 px-4 py-2.5 flex justify-between items-center text-xs",
                    div { class: "flex items-center gap-2",
                        components::LucideIcon { name: "package", size: "14", class: "text-primary" }
                        div {
                            div { class: "font-extrabold text-foreground", 
                                "{t(\"booking-widget-volume-label\", &loc)}: {total_volume:.1} m³" 
                            }
                            div { class: "text-[9px] text-muted-foreground", "{t(\"booking-widget-capacity-sub\", &loc)}" }
                        }
                    }
                    div { class: "text-right",
                        div { class: "font-black text-primary text-sm", "{estimated_price:.0} SEK" }
                        div { class: "text-[9px] text-muted-foreground", "{t(\"booking-widget-estimated-price\", &loc)}" }
                    }
                }
            }

            // Step Content
            div { class: "p-5 flex-1 flex flex-col justify-between gap-5",
                if *step.read() == 1 {
                    div { class: "space-y-4 flex-1",
                        h4 { class: "text-xs font-bold text-muted-foreground uppercase tracking-wider mb-1", "{t(\"booking-widget-step1-title\", &loc)}" }
                        
                        div { class: "space-y-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-bold text-foreground", "Namn *" }
                                input {
                                    class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                    placeholder: "Förnamn Efternamn",
                                    value: "{name}",
                                    oninput: move |e| name.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-bold text-foreground", "E-postadress *" }
                                input {
                                    class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                    placeholder: "namn@epost.se",
                                    r#type: "email",
                                    value: "{email}",
                                    oninput: move |e| email.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-bold text-foreground", "Telefonnummer *" }
                                input {
                                    class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                    placeholder: "070-123 45 67",
                                    r#type: "tel",
                                    value: "{phone}",
                                    oninput: move |e| phone.set(e.value())
                                }
                            }
                        }
                    }
                } else if *step.read() == 2 {
                    div { class: "space-y-4 flex-1",
                        h4 { class: "text-xs font-bold text-muted-foreground uppercase tracking-wider mb-1", "{t(\"booking-widget-step2-title\", &loc)}" }
                        
                        div { class: "space-y-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-bold text-foreground", "Från Adress (Nuvarande) *" }
                                input {
                                    class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                    placeholder: "Gata 12, Stad",
                                    value: "{origin}",
                                    oninput: move |e| origin.set(e.value())
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[10px] font-bold text-foreground", "Till Adress (Nya) *" }
                                input {
                                    class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                    placeholder: "Nya Vägen 45, Stad",
                                    value: "{destination}",
                                    oninput: move |e| destination.set(e.value())
                                }
                            }
                        }
                    }
                } else if *step.read() == 3 {
                    div { class: "flex flex-col gap-3.5 flex-1",
                        h4 { class: "text-xs font-bold text-muted-foreground uppercase tracking-wider mb-0", "{t(\"booking-widget-step3-title\", &loc)}" }

                        // Presets Quick Add list
                        div { class: "space-y-1.5",
                            span { class: "text-[9px] font-bold text-muted-foreground uppercase tracking-wide", "{t(\"booking-widget-quick-add\", &loc)}" }
                            div { class: "flex flex-wrap gap-1.5 max-h-24 overflow-y-auto pr-1",
                                for (name_pr, vol_pr) in presets.into_iter() {
                                    button {
                                        key: "{name_pr}",
                                        class: "text-[10px] font-semibold px-2 py-1 rounded-full border border-border bg-background hover:bg-secondary/20 hover:border-primary/40 cursor-pointer text-foreground transition-all flex items-center gap-1",
                                        onclick: move |_| {
                                            let mut current = selected_items.read().clone();
                                            if let Some(pos) = current.iter().position(|i| i.name == name_pr) {
                                                current[pos].quantity += 1;
                                            } else {
                                                current.push(WidgetItem {
                                                    name: name_pr.to_string(),
                                                    volume: vol_pr,
                                                    quantity: 1,
                                                });
                                            }
                                            selected_items.set(current);
                                        },
                                        components::LucideIcon { name: "plus", size: "10" }
                                        "{name_pr}"
                                    }
                                }
                            }
                            // Custom Item Entry Form
                            div { class: "flex items-center gap-1.5 pt-1",
                                input {
                                    class: "flex-1 rounded-md border border-border bg-background px-2.5 py-1 text-[11px] text-foreground focus:outline-none focus:border-primary",
                                    placeholder: "{t(\"booking-widget-custom-item\", &loc)}",
                                    value: "{custom_item_name}",
                                    oninput: move |e| custom_item_name.set(e.value())
                                }
                                input {
                                    r#type: "number",
                                    step: "0.1",
                                    class: "w-16 rounded-md border border-border bg-background px-2 py-1 text-[11px] text-foreground focus:outline-none focus:border-primary",
                                    placeholder: "m³",
                                    value: "{custom_item_vol}",
                                    oninput: move |e| custom_item_vol.set(e.value().parse().unwrap_or(0.5))
                                }
                                button {
                                    class: "px-2.5 py-1 rounded-md bg-primary text-primary-foreground text-[10px] font-bold cursor-pointer hover:opacity-90 border-0 flex items-center gap-1",
                                    onclick: move |_| {
                                        let name_val = custom_item_name.read().trim().to_string();
                                        let vol_val = *custom_item_vol.read();
                                        if !name_val.is_empty() {
                                            let mut current = selected_items.read().clone();
                                            current.push(WidgetItem {
                                                name: name_val,
                                                volume: vol_val,
                                                quantity: 1,
                                            });
                                            selected_items.set(current);
                                            custom_item_name.set(String::new());
                                        }
                                    },
                                    components::LucideIcon { name: "plus", size: "10" }
                                    "{t(\"booking-widget-add-btn\", &loc)}"
                                }
                            }
                        }

                        // Selected Items list with inline editing & removal
                        div { class: "flex-1 border border-border/40 bg-background/50 rounded-xl p-3 flex flex-col justify-stretch min-h-[140px]",
                            span { class: "text-[9px] font-bold text-muted-foreground uppercase tracking-wide mb-1", "{t(\"booking-widget-selected-items\", &loc)}" }
                            
                            if items_list.is_empty() {
                                div { class: "flex-1 flex flex-col items-center justify-center text-center text-muted-foreground/60 py-4",
                                    components::LucideIcon { name: "box", class: "h-5 w-5 opacity-40 mb-1" }
                                    p { class: "text-[10px] m-0 font-medium", "{t(\"booking-widget-no-items\", &loc)}" }
                                }
                            } else {
                                div { class: "flex-1 overflow-y-auto max-h-36 space-y-1.5 pr-1",
                                    for (idx, item) in items_list.iter().enumerate() {
                                        div {
                                            key: "{idx}",
                                            class: "flex justify-between items-center bg-background border border-border/30 p-2 rounded-lg text-xs gap-2 shadow-sm",
                                            
                                            // Inline editable item name & volume
                                            div { class: "flex items-center gap-1.5 flex-1 min-w-0",
                                                input {
                                                    class: "text-xs font-semibold text-foreground bg-transparent border border-transparent hover:border-border/40 focus:border-primary focus:bg-background rounded px-1 py-0.5 truncate w-full transition-all",
                                                    value: "{item.name}",
                                                    oninput: move |e| {
                                                        let new_name = e.value();
                                                        let mut current = selected_items.read().clone();
                                                        if idx < current.len() {
                                                            current[idx].name = new_name;
                                                            selected_items.set(current);
                                                        }
                                                    }
                                                }
                                                div { class: "flex items-center gap-1 shrink-0",
                                                    input {
                                                        r#type: "number",
                                                        step: "0.1",
                                                        class: "w-12 text-[10px] font-mono text-muted-foreground bg-transparent border border-transparent hover:border-border/40 focus:border-primary focus:bg-background rounded px-1 py-0.5 text-right transition-all",
                                                        value: "{item.volume}",
                                                        oninput: move |e| {
                                                            let new_vol = e.value().parse().unwrap_or(0.5);
                                                            let mut current = selected_items.read().clone();
                                                            if idx < current.len() {
                                                                current[idx].volume = new_vol;
                                                                selected_items.set(current);
                                                            }
                                                        }
                                                    }
                                                    span { class: "text-[9px] text-muted-foreground font-semibold", "m³" }
                                                }
                                            }

                                            // Quantity Selector & Trash Action
                                            div { class: "flex items-center gap-1.5 shrink-0",
                                                div { class: "flex items-center border border-border rounded overflow-hidden bg-background shadow-xs",
                                                    button {
                                                        class: "p-1 border-0 hover:bg-muted text-foreground cursor-pointer flex items-center justify-center bg-transparent",
                                                        onclick: move |_| {
                                                            let mut current = selected_items.read().clone();
                                                            if idx < current.len() {
                                                                if current[idx].quantity > 1 {
                                                                    current[idx].quantity -= 1;
                                                                } else {
                                                                    current.remove(idx);
                                                                }
                                                                selected_items.set(current);
                                                            }
                                                        },
                                                        components::LucideIcon { name: "minus", size: "9" }
                                                    }
                                                    span { class: "px-2 text-[10px] font-bold text-foreground min-w-[16px] text-center", "{item.quantity}" }
                                                    button {
                                                        class: "p-1 border-0 hover:bg-muted text-foreground cursor-pointer flex items-center justify-center bg-transparent",
                                                        onclick: move |_| {
                                                            let mut current = selected_items.read().clone();
                                                            if idx < current.len() {
                                                                current[idx].quantity += 1;
                                                                selected_items.set(current);
                                                            }
                                                        },
                                                        components::LucideIcon { name: "plus", size: "9" }
                                                    }
                                                }
                                                button {
                                                    class: "p-1 border-0 hover:bg-rose-500/10 text-slate-400 hover:text-rose-500 rounded cursor-pointer transition-all flex items-center justify-center bg-transparent",
                                                    onclick: move |_| {
                                                        let mut current = selected_items.read().clone();
                                                        if idx < current.len() {
                                                            current.remove(idx);
                                                            selected_items.set(current);
                                                        }
                                                    },
                                                    components::LucideIcon { name: "trash-2", size: "12" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if *step.read() == 4 {
                    // Success View!
                    div { class: "flex-1 flex flex-col items-center justify-center text-center gap-3 py-8 animate-in fade-in duration-300",
                        div { class: "h-12 w-12 rounded-full bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 flex items-center justify-center",
                            components::LucideIcon { name: "check-circle", size: "28" }
                        }
                        div {
                            h4 { class: "text-sm font-black text-foreground m-0", "{t(\"booking-widget-step4-title\", &loc)}" }
                            p { class: "text-xs text-muted-foreground mt-1 max-w-sm mx-auto leading-relaxed", 
                                "Vi har tagit emot dina uppgifter och skickat en bekräftelse till e-postadressen. Våra handläggare kommer att granska din förfrågan inom kort." 
                            }
                        }
                        if let Some(ref j_id) = created_job_id.read().as_ref() {
                            div { class: "bg-muted p-2 rounded-lg font-mono text-[9px] text-muted-foreground font-semibold border border-border/40",
                                "Referens-ID: {j_id}"
                            }
                        }
                    }
                }

                // Error alert
                if let Some(ref err) = submit_error.read().as_ref() {
                    div { class: "bg-red-500/10 border border-red-500/20 text-red-500 rounded-lg p-2.5 text-[10px] font-bold text-center leading-relaxed",
                        "⚠️ {err}"
                    }
                }

                // Navigation Buttons (Footer)
                if *step.read() < 4 {
                    div { class: "flex justify-between items-center gap-3 pt-3 border-t border-border/30 mt-auto",
                        if *step.read() > 1 {
                            button {
                                class: "px-4 py-2 rounded-lg border border-border bg-background hover:bg-muted text-xs font-bold text-foreground cursor-pointer transition-all",
                                onclick: move |_| {
                                    let current = *step.read();
                                    step.set(current - 1);
                                },
                                "{t(\"booking-widget-back\", &loc)}"
                            }
                        } else {
                            div {}
                        }
                        
                        if *step.read() < 3 {
                            button {
                                class: "px-5 py-2 rounded-lg bg-primary text-primary-foreground hover:opacity-90 text-xs font-black border-0 cursor-pointer shadow transition-all",
                                onclick: move |_| {
                                    let current = *step.read();
                                    step.set(current + 1);
                                },
                                "{t(\"booking-widget-next\", &loc)}"
                            }
                        } else {
                            button {
                                class: "px-6 py-2 rounded-lg bg-emerald-600 text-white hover:opacity-90 text-xs font-black border-0 cursor-pointer shadow transition-all disabled:opacity-50 disabled:cursor-not-allowed",
                                disabled: submitting.read().clone(),
                                onclick: handle_submit,
                                if *submitting.read() { "Skickar..." } else { "{t(\"booking-widget-send\", &loc)}" }
                            }
                        }
                    }
                } else {
                    // Start over / Success Button
                    div { class: "flex justify-center pt-2 mt-auto",
                        button {
                            class: "px-6 py-2 rounded-lg bg-primary text-primary-foreground hover:opacity-90 text-xs font-bold border-0 cursor-pointer transition-all shadow",
                            onclick: move |_| {
                                name.set(String::new());
                                email.set(String::new());
                                phone.set(String::new());
                                origin.set(String::new());
                                destination.set(String::new());
                                selected_items.set(Vec::new());
                                created_job_id.set(None);
                                submit_error.set(None);
                                step.set(1);
                            },
                            "Gör en ny beräkning"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn PublicBookingPreview(workspace_id: String, locale: Option<String>) -> Element {
    let ws_id = workspace_id.clone();
    let loc = locale.clone();
    let mut view_tab = use_signal(|| "preview".to_string()); // "preview" or "embed"

    // Simple raw HTML embed snippet
    let iframe_code = format!(
        "<iframe src=\"https://yntra.se/embed/booking/{}\" width=\"100%\" height=\"600\" style=\"border:none; border-radius:16px; box-shadow: 0 10px 15px -3px rgba(0,0,0,0.1);\"></iframe>",
        ws_id
    );

    rsx! {
        div { class: "flex flex-col gap-6 w-full animate-in fade-in duration-300",
            
            // Preview Panel Header with View Mode Pills
            div { class: "flex justify-between items-center border-b border-border/40 pb-3 flex-wrap gap-3",
                div {
                    h3 { class: "text-sm font-extrabold m-0 text-foreground", "Embeddbar Lead-Widget" }
                    p { class: "text-[10px] text-muted-foreground mt-1 mb-0", "Integrera bokningsformuläret på er hemsida för att ta emot förfrågningar automatiskt." }
                }

                // View Mode Toggle Pills
                div { class: "flex items-center gap-1 bg-sidebar border border-border p-1 rounded-2xl shadow-sm",
                    button {
                        onclick: move |_| view_tab.set("preview".to_string()),
                        class: format!("px-3 py-1.5 rounded-xl text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *view_tab.read() == "preview" { "bg-primary text-primary-foreground shadow" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                        components::LucideIcon { name: "play", size: "12" }
                        "Förhandsgranskning"
                    }
                    button {
                        onclick: move |_| view_tab.set("embed".to_string()),
                        class: format!("px-3 py-1.5 rounded-xl text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *view_tab.read() == "embed" { "bg-primary text-primary-foreground shadow" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                        components::LucideIcon { name: "code", size: "12" }
                        "Embedd-kod & Inställningar"
                    }
                }
            }

            // Tab 1: Interactive Live Preview
            if *view_tab.read() == "preview" {
                div { class: "flex gap-6 items-stretch flex-wrap md:flex-nowrap animate-in fade-in duration-200",
                    
                    // Instructions Info Card
                    div { class: "flex-1 min-w-[300px] border border-border bg-sidebar rounded-2xl p-6 flex flex-col justify-between shadow-sm",
                        div { class: "space-y-4",
                            h4 { class: "text-xs font-black text-foreground uppercase tracking-wider m-0", "Hur det fungerar" }
                            
                            div { class: "space-y-3 text-xs leading-relaxed text-muted-foreground",
                                div { class: "flex gap-3 items-start",
                                    div { class: "h-5 w-5 rounded-full bg-primary/10 border border-primary/20 text-primary font-bold flex items-center justify-center text-[10px] flex-shrink-0 mt-0.5", "1" }
                                    div {
                                        div { class: "font-bold text-foreground", "Embedda på din sajt" }
                                        p { class: "text-[10px] mt-0.5 m-0", "Kopiera iframe-koden och klistra in på företagets hemsida." }
                                    }
                                }
                                div { class: "flex gap-3 items-start",
                                    div { class: "h-5 w-5 rounded-full bg-primary/10 border border-primary/20 text-primary font-bold flex items-center justify-center text-[10px] flex-shrink-0 mt-0.5", "2" }
                                    div {
                                        div { class: "font-bold text-foreground", "Kunden kalkylerar själv" }
                                        p { class: "text-[10px] mt-0.5 m-0", "Besökaren matar in flyttartiklar och får ett direkt pris- och volymförslag i realtid." }
                                    }
                                }
                                div { class: "flex gap-3 items-start",
                                    div { class: "h-5 w-5 rounded-full bg-primary/10 border border-primary/20 text-primary font-bold flex items-center justify-center text-[10px] flex-shrink-0 mt-0.5", "3" }
                                    div {
                                        div { class: "font-bold text-foreground", "Automatiskt i planeringen" }
                                        p { class: "text-[10px] mt-0.5 m-0", "När förfrågan skickas skapas ett jobb i databasen och uppdraget dyker upp i planeringen." }
                                    }
                                }
                            }
                        }

                        // Bottom info stats
                        div { class: "border-t border-border/30 pt-4 mt-6 flex justify-around text-center text-xs",
                            div {
                                div { class: "font-black text-foreground text-sm", "2.5 m³" }
                                div { class: "text-[9px] text-muted-foreground", "Snittvolym" }
                            }
                            div {
                                div { class: "font-black text-foreground text-sm", "100%" }
                                div { class: "text-[9px] text-muted-foreground", "Automatiskt" }
                            }
                            div {
                                div { class: "font-black text-foreground text-sm", "5 Språk" }
                                div { class: "text-[9px] text-muted-foreground", "Språkstöd" }
                            }
                        }
                    }

                    // Widget Preview frame
                    div { class: "flex-1 min-w-[320px] flex justify-center items-center py-6 bg-gradient-to-br from-secondary/5 via-primary/5 to-accent/5 border border-border border-dashed rounded-2xl relative",
                        PublicBookingWidget { workspace_id: ws_id, locale: loc }
                    }
                }
            } else {
                // Tab 2: Embed Code & Settings
                div { class: "bg-sidebar border border-border rounded-2xl p-6 space-y-5 animate-in slide-in-from-bottom-2 duration-200 shadow-sm",
                    div { class: "space-y-1",
                        h4 { class: "text-sm font-black text-foreground m-0", "HTML Iframe Embed-kod" }
                        p { class: "text-xs text-muted-foreground m-0", "Kopiera koden nedan och klistra in i er webbplatssida (WordPress, Webflow, Wix, Shopify eller anpassad HTML)." }
                    }

                    pre { class: "text-xs font-mono p-4 bg-background rounded-xl border border-border/60 overflow-x-auto whitespace-pre-wrap select-all text-foreground m-0 shadow-inner",
                        "{iframe_code}"
                    }

                    div { class: "flex items-center gap-3 pt-2",
                        button {
                            class: "px-4 py-2 rounded-lg bg-primary text-primary-foreground hover:opacity-90 text-xs font-extrabold border-0 cursor-pointer transition-all shadow flex items-center gap-1.5",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    if let Some(window) = web_sys::window() {
                                        if let Some(navigator) = window.navigator().clipboard() {
                                            let _ = navigator.write_text(&iframe_code);
                                        }
                                    }
                                }
                            },
                            components::LucideIcon { name: "save", size: "14" }
                            "Kopiera Embedd-kod"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
pub struct BookingWidgetViewProps {
    pub active_user_id: Signal<String>,
    pub auth_region: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for BookingWidgetViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn BookingWidgetView(props: BookingWidgetViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let region = props.auth_region.read().clone();
    let workspace_opt = state.workspace.read().clone();
    let workspace_id = if let Some(ref ws) = workspace_opt {
        ws.id.clone()
    } else {
        "workspace-1".to_string()
    };

    rsx! {
        div { class: "mx-auto w-full max-w-5xl p-6 flex flex-col gap-6",
            div { class: "flex flex-col gap-1",
                h1 { class: "text-2xl font-bold text-foreground", "Boknings-widget" }
                p { class: "text-sm text-muted-foreground", "Generera och förhandsgranska embeddbar boknings-widget för er hemsida." }
            }
            PublicBookingPreview { workspace_id: workspace_id, locale: Some(region) }
        }
    }
}
