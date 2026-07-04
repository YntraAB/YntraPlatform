use dioxus::prelude::*;
use yntra_core::{get_job_tickets, get_move_inventory, get_move_quote, accept_move_quote};
use crate::components;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Props, Clone, PartialEq)]
pub struct MovingPortalProps {
    pub active_user_id: String,
    pub db_trigger: Signal<u32>,
}

#[component]
pub fn MovingPortal(props: MovingPortalProps) -> Element {
    let mut db_trigger = props.db_trigger;

    let mut active_tab = use_signal(|| "moving_jobs".to_string());

    // Fetch resources locally
    let jobs_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = props.active_user_id.clone();
        async move { get_job_tickets(uid).await.unwrap_or_default() }
    });
    let jobs = jobs_res.read().clone().unwrap_or_default();

    let active_job = jobs.first().cloned();
    let active_job_id = active_job.as_ref().map(|j| j.id.clone()).unwrap_or_default();
    
    let active_job_id_for_inv = active_job_id.clone();
    let inventory_res = use_resource(move || {
        let _trig = db_trigger.read();
        let jid = active_job_id_for_inv.clone();
        async move {
            if jid.is_empty() {
                Vec::new()
            } else {
                get_move_inventory(jid).await.unwrap_or_default()
            }
        }
    });
    
    let active_job_id_for_quote = active_job_id.clone();
    let quote_res = use_resource(move || {
        let _trig = db_trigger.read();
        let jid = active_job_id_for_quote.clone();
        async move {
            if jid.is_empty() {
                None
            } else {
                get_move_quote(jid).await.unwrap_or(None)
            }
        }
    });

    let inventories = inventory_res.read().clone().unwrap_or_default();
    let total_volume: f64 = inventories.iter().map(|i| i.estimated_volume_m3 * i.quantity as f64).sum();
    let quote = quote_res.read().clone().flatten();

    let on_accept_quote = move |quote_id: String| {
        spawn(async move {
            if accept_move_quote(quote_id).await.is_ok() {
                let current_val = *db_trigger.read();
                db_trigger.set(current_val + 1);
            }
        });
    };

    rsx! {
        // 2. Moving Company Portal Widgets
        div { class: "grid grid-cols-1 gap-6 md:grid-cols-3",
            
            // Left/Main Card: Inbokade Flyttar
            div { class: "md:col-span-2",
                components::Card {
                    class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                    components::CardHeader {
                        class: "pb-3 border-b border-border/40",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                components::LucideIcon { name: "truck", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold", "Inbokade Flyttar" }
                                components::CardDescription { class: "text-xs", "Aktuell status för dina flyttuppdrag." }
                            }
                        }
                    }
                    components::CardContent {
                        class: "pt-4 flex flex-1 flex-col justify-between",
                        if jobs.is_empty() {
                            div { class: "flex flex-col items-center justify-center py-8 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/20 my-auto",
                                components::LucideIcon { name: "truck", class: "h-8 w-8 opacity-20 mb-2" }
                                p { class: "text-sm m-0", "Inga flyttar inbokade för tillfället." }
                            }
                        } else {
                            div { class: "space-y-4",
                                for job in jobs.iter() {
                                    div {
                                        key: "{job.id}",
                                        class: "flex flex-col gap-3 rounded-xl border border-border/30 bg-background/50 p-4 transition-all hover:bg-secondary/20",
                                        div { class: "flex items-start gap-3",
                                            div { class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary font-bold",
                                                components::LucideIcon { name: "truck", class: "h-5 w-5" }
                                            }
                                            div { class: "flex-1 min-w-0",
                                                h4 { class: "text-sm font-bold text-foreground truncate m-0", "{job.title}" }
                                                p { class: "text-xs text-muted-foreground mt-0.5 line-clamp-2 m-0", "{job.description}" }
                                                div { class: "flex items-center gap-1.5 text-xs text-primary font-semibold mt-1",
                                                    components::LucideIcon { name: "calendar", class: "h-3.5 w-3.5" }
                                                    span { "{job.scheduled_date}" }
                                                }
                                            }
                                        }
                                        if let (Some(ref origin), Some(ref dest)) = (job.origin_address.clone(), job.destination_address.clone()) {
                                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 border-t border-border/20 pt-3 mt-1",
                                                div { class: "space-y-1",
                                                    div { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Från (Ursprung)" }
                                                    div { class: "text-xs font-semibold text-foreground", "{origin}" }
                                                    div { class: "text-[10px] text-muted-foreground",
                                                        "Våning {job.origin_floor} • Hiss: "
                                                        if job.origin_has_elevator { "Ja" } else { "Nej" }
                                                        " • Parkering: "
                                                        if job.origin_parking_permit_needed { "Krävs" } else { "Ej krav" }
                                                    }
                                                }
                                                div { class: "space-y-1",
                                                    div { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Till (Destination)" }
                                                    div { class: "text-xs font-semibold text-foreground", "{dest}" }
                                                    div { class: "text-[10px] text-muted-foreground",
                                                        "Våning {job.destination_floor} • Hiss: "
                                                        if job.destination_has_elevator { "Ja" } else { "Nej" }
                                                        " • Parkering: "
                                                        if job.destination_parking_permit_needed { "Krävs" } else { "Ej krav" }
                                                    }
                                                }
                                            }
                                        } else {
                                            div { class: "flex items-center gap-1 text-xs text-muted-foreground border-t border-border/20 pt-2",
                                                components::LucideIcon { name: "map-pin", class: "h-3.5 w-3.5 text-primary" }
                                                span { "{job.location_address}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Right Card: Ditt Offer / Price Calculation or Flyttöversikt
            div { class: "col-span-1",
                if let Some(ref q) = quote {
                    components::Card {
                        class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                        components::CardHeader {
                            class: "pb-3 border-b border-border/40",
                            div { class: "flex items-center gap-3",
                                div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                    components::LucideIcon { name: "credit-card", class: "h-5 w-5" }
                                }
                                div {
                                    components::CardTitle { class: "text-lg font-bold", "Ditt Priserbjudande" }
                                    components::CardDescription { class: "text-xs", "Prisberäkning för din flytt." }
                                }
                            }
                        }
                        components::CardContent {
                            class: "pt-6 space-y-4 flex-1 flex flex-col justify-between",
                            div { class: "space-y-2",
                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                    span { "Baspris (arbete/tid):" }
                                    span { class: "font-semibold text-foreground", "{q.base_price} kr" }
                                }
                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                    span { "Distansavgift:" }
                                    span { class: "font-semibold text-foreground", "{q.distance_fee} kr" }
                                }
                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                    span { "Trapptillägg:" }
                                    span { class: "font-semibold text-foreground", "{q.stairs_surcharge} kr" }
                                }
                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                    span { "Packmaterial & utrustning:" }
                                    span { class: "font-semibold text-foreground", "{q.packing_supplies_fee} kr" }
                                }
                                div { class: "h-px bg-border/40 my-2", }
                                div { class: "flex items-center justify-between text-sm font-extrabold",
                                    span { "Totalt pris:" }
                                    span { class: "text-primary text-base", "{q.total_price} kr" }
                                }
                            }
                            
                            // Offer acceptance action
                            div { class: "pt-4",
                                if q.status == "accepted" {
                                    div { class: "flex items-center justify-center gap-2 p-2.5 rounded-lg bg-emerald-500/10 text-emerald-500 text-xs font-bold text-center border border-emerald-500/20",
                                        components::LucideIcon { name: "check-circle", class: "h-4 w-4" }
                                        span { "Offert Godkänd" }
                                    }
                                } else {
                                    button {
                                        onclick: {
                                            let q_id = q.id.clone();
                                            move |_| on_accept_quote(q_id.clone())
                                        },
                                        class: "w-full rounded-lg bg-primary py-2.5 text-xs font-bold text-primary-foreground shadow hover:opacity-90 transition-all border-0 cursor-pointer flex items-center justify-center gap-2",
                                        components::LucideIcon { name: "thumbs-up", class: "h-4 w-4" }
                                        "Godkänn Flytoffert"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    components::Card {
                        class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                        components::CardHeader {
                            class: "pb-3 border-b border-border/40",
                            div { class: "flex items-center gap-3",
                                div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                    components::LucideIcon { name: "package", class: "h-5 w-5" }
                                }
                                div {
                                    components::CardTitle { class: "text-lg font-bold", "Flyttöversikt" }
                                    components::CardDescription { class: "text-xs", "Bokningsstatistik för din flytt." }
                                }
                            }
                        }
                        components::CardContent {
                            class: "pt-6 space-y-4 flex-1 flex flex-col justify-center",
                            div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                                span { class: "text-xs font-semibold text-muted-foreground", "Antal flyttuppdrag" }
                                span { class: "text-sm font-bold text-foreground", "{jobs.len()} st" }
                            }
                            div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                                span { class: "text-xs font-semibold text-muted-foreground", "Volymuppskattning" }
                                span { class: "text-sm font-bold text-foreground", "0 m³" }
                            }
                            div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                                span { class: "text-xs font-semibold text-muted-foreground", "Offertstatus" }
                                span { class: "text-sm font-bold text-muted-foreground", "Ej skapad" }
                            }
                        }
                    }
                }
            }
        }

        // Tabs Section for Moving
        div { class: "w-full animate-in fade-in zoom-in duration-300",
            div { class: "grid w-full max-w-md grid-cols-2 bg-muted/50 rounded-xl p-1 mb-6 border border-border",
                button {
                    onclick: move |_| active_tab.set("moving_jobs".to_string()),
                    class: if *active_tab.read() == "moving_jobs" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "check-square", class: "h-3.5 w-3.5" }
                    "Mina Flyttuppdrag"
                }
                button {
                    onclick: move |_| active_tab.set("moving_agreement".to_string()),
                    class: if *active_tab.read() == "moving_agreement" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "file-text", class: "h-3.5 w-3.5" }
                    "Avtal & Detaljer"
                }
            }

            if *active_tab.read() == "moving_jobs" {
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                    // Checklist: Left/Main 2 cols
                    div { class: "md:col-span-2",
                        components::Card {
                            class: "border border-border bg-sidebar shadow-md h-full",
                            components::CardHeader {
                                class: "pb-3 border-b border-border/40",
                                components::CardTitle { class: "text-base font-bold", "Checklista Flyttdag" }
                                components::CardDescription { class: "text-xs", "Uppgifter att slutföra inför och under flyttdagen." }
                            },
                            components::CardContent {
                                class: "pt-6",
                                if jobs.is_empty() {
                                    div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10",
                                        components::LucideIcon { name: "check-square", class: "h-8 w-8 opacity-20 mb-2" }
                                        p { class: "text-sm font-semibold m-0", "Inga checklistor tillgängliga." }
                                    }
                                } else {
                                    div { class: "space-y-3",
                                        for job in jobs.iter() {
                                            {
                                                let checklist_items: Vec<ChecklistItem> = serde_json::from_str(&job.checklist_json).unwrap_or_default();
                                                rsx! {
                                                    div {
                                                        key: "{job.id}",
                                                        class: "space-y-2",
                                                        for item in checklist_items.iter() {
                                                            div { class: "flex items-center gap-3 p-3 rounded-lg bg-background/50 border border-border/20 transition-all hover:bg-secondary/10",
                                                                components::LucideIcon {
                                                                    name: if item.done { "check-circle" } else { "circle" },
                                                                    class: if item.done { "h-4 w-4 text-emerald-500" } else { "h-4 w-4 text-muted-foreground/75" }
                                                                }
                                                                span {
                                                                    class: "text-xs font-medium",
                                                                    style: if item.done { "text-decoration: line-through; color: var(--text-secondary);" } else { "" },
                                                                    "{item.text}"
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
                        }
                    },
                    // Inventory: Right 1 col
                    div { class: "col-span-1",
                        components::Card {
                            class: "border border-border bg-sidebar shadow-md h-full flex flex-col",
                            components::CardHeader {
                                class: "pb-3 border-b border-border/40",
                                components::CardTitle { class: "text-base font-bold", "Flyttinventarie & Volym" }
                                components::CardDescription { class: "text-xs", "Bohag uppdelat per kategori." }
                            },
                            components::CardContent {
                                class: "pt-6 flex-1 flex flex-col justify-between h-full",
                                if inventories.is_empty() {
                                    div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10 my-auto",
                                        components::LucideIcon { name: "box", class: "h-8 w-8 opacity-20 mb-2" }
                                        p { class: "text-sm font-semibold m-0", "Inget inventarie tillagt." }
                                    }
                                } else {
                                    div { class: "space-y-3 flex-1 overflow-y-auto max-h-[300px] pr-1",
                                        for item in inventories.iter() {
                                            div {
                                                key: "{item.id}",
                                                class: "flex items-start justify-between p-2.5 rounded-lg bg-background/50 border border-border/10",
                                                div { class: "min-w-0 flex-1",
                                                    div { class: "text-xs font-bold text-foreground truncate", "{item.item_name}" }
                                                    div { class: "text-[10px] text-muted-foreground", "{item.item_category} • {item.estimated_volume_m3} m³" }
                                                }
                                                div { class: "text-right pl-2",
                                                    div { class: "text-xs font-extrabold text-primary", "{item.quantity}x" }
                                                    div { class: "text-[10px] text-muted-foreground font-semibold", "{item.estimated_volume_m3 * item.quantity as f64} m³" }
                                                }
                                            }
                                        }
                                    }
                                    
                                    div { class: "border-t border-border/30 pt-3 mt-2 flex items-center justify-between",
                                        span { class: "text-xs font-bold text-muted-foreground", "Total cargo-volym:" }
                                        span { class: "text-sm font-extrabold text-primary", "{total_volume:.1} m³" }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                components::Card {
                    class: "border border-border bg-sidebar shadow-md",
                    components::CardContent {
                        class: "pt-6 space-y-6 flex flex-col gap-4",
                        div { class: "border-b border-border/30 pb-4",
                            h3 { class: "text-base font-bold text-foreground m-0", "Standardavtal för Flyttjänster" }
                            p { class: "text-xs text-muted-foreground mt-1 m-0", "Senast uppdaterat: 2026-07-04" }
                        }
                        div { class: "text-sm text-foreground leading-relaxed whitespace-pre-line bg-background/30 p-4 rounded-xl border border-border/20 m-0",
                            "1. OMFATTNING\nTjänsten omfattar flytt av bohag enligt den godkända inventarielistan. Packningsmaterial tillhandahålls av flyttfirman om detta överenskommits.\n\n2. ANSVAR & FÖRSÄKRING\nFöretaget ansvarar för skador som uppkommer på grund av vårdslöshet från personalens sida. Kunden uppmanas att teckna en kompletterande hemförsäkring med flyttskydd. Allt bohag är försäkrat under transporten upp till 500 000 SEK.\n\n3. AVBOKNINGSREGLER\nAvbokning kostnadsfritt fram till 5 arbetsdagar innan bokat flyttdatum. Därefter debiteras 50% av den estimerade arbestaytan."
                        }
                    }
                }
            }
        }
    }
}
