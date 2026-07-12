use dioxus::prelude::*;
use yntra_core::get_job_tickets_rkyv;
use crate::components;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Props, Clone, PartialEq)]
pub struct GeneralPortalProps {
    pub active_user_id: String,
    pub db_trigger: Signal<u32>,
}

#[component]
pub fn GeneralPortal(props: GeneralPortalProps) -> Element {
    let db_trigger = props.db_trigger;

    let mut active_tab = use_signal(|| "general_jobs".to_string());

    // Fetch resources locally
    let jobs_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = props.active_user_id.clone();
        async move {
            match get_job_tickets_rkyv(uid).await {
                Ok(bytes) => {
                    rkyv::from_bytes::<Vec<yntra_core::JobTicket>, rkyv::rancor::Error>(&bytes).unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        }
    });
    let jobs = jobs_res.read().clone().unwrap_or_default();

    rsx! {
        // 3. General Operations Portal Widgets
        div { class: "grid grid-cols-1 gap-6 md:grid-cols-3",
            
            // Left/Main Card: Aktiva Uppdrag
            div { class: "md:col-span-2",
                components::Card {
                    class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                    components::CardHeader {
                        class: "pb-3 border-b border-border/40",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                components::LucideIcon { name: "wrench", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold", "Aktiva Uppdrag" }
                                components::CardDescription { class: "text-xs", "Aktuella service- och arbetsordrar." }
                            }
                        }
                    }
                    components::CardContent {
                        class: "pt-4 flex flex-1 flex-col justify-between",
                        if jobs.is_empty() {
                            div { class: "flex flex-col items-center justify-center py-8 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/20 my-auto",
                                components::LucideIcon { name: "wrench", class: "h-8 w-8 opacity-20 mb-2" }
                                p { class: "text-sm m-0", "Inga aktiva uppdrag." }
                            }
                        } else {
                            div { class: "space-y-4",
                                for job in jobs.iter() {
                                    div {
                                        key: "{job.id}",
                                        class: "flex flex-col gap-3 rounded-xl border border-border/30 bg-background/50 p-4 transition-all hover:bg-secondary/20",
                                        div { class: "flex items-start gap-3",
                                            div { class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary font-bold",
                                                components::LucideIcon { name: "check-circle", class: "h-4 w-4" }
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

            // Right Card: Uppdragsöversikt Stats
            div { class: "col-span-1",
                components::Card {
                    class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                    components::CardHeader {
                        class: "pb-3 border-b border-border/40",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                components::LucideIcon { name: "layout-grid", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold", "Uppdragsöversikt" }
                                components::CardDescription { class: "text-xs", "Sammanställning av dina ärenden." }
                            }
                        }
                    }
                    components::CardContent {
                        class: "pt-6 space-y-4 flex-1 flex flex-col justify-center",
                        div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                            span { class: "text-xs font-semibold text-muted-foreground", "Aktiva Ärenden" }
                            span { class: "text-sm font-bold text-foreground", "{jobs.len()} st" }
                        }
                        div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                            span { class: "text-xs font-semibold text-muted-foreground", "Slutförda Ärenden" }
                            span { class: "text-sm font-bold text-foreground", "12 st" }
                        }
                        div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                            span { class: "text-xs font-semibold text-muted-foreground", "Medlemsnivå" }
                            span { class: "text-sm font-bold text-primary", "Företagskund" }
                        }
                    }
                }
            }
        }

        // Tabs Section for General Operations
        div { class: "w-full animate-in fade-in zoom-in duration-300",
            div { class: "grid w-full max-w-md grid-cols-2 bg-muted/50 rounded-xl p-1 mb-6 border border-border",
                button {
                    onclick: move |_| active_tab.set("general_jobs".to_string()),
                    class: if *active_tab.read() == "general_jobs" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "wrench", class: "h-3.5 w-3.5" }
                    "Mina Uppdrag"
                }
                button {
                    onclick: move |_| active_tab.set("general_support".to_string()),
                    class: if *active_tab.read() == "general_support" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "life-buoy", class: "h-3.5 w-3.5" }
                    "Support & Kontakt"
                }
            }

            if *active_tab.read() == "general_jobs" {
                components::Card {
                    class: "border border-border bg-sidebar shadow-md",
                    components::CardContent {
                        class: "pt-6",
                        if jobs.is_empty() {
                            div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10",
                                components::LucideIcon { name: "wrench", class: "h-8 w-8 opacity-20 mb-2" }
                                p { class: "text-sm font-semibold m-0", "Inga uppdrag listade." }
                            }
                        } else {
                            div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                                for job in jobs.iter() {
                                    {
                                        let checklist_items: Vec<ChecklistItem> = serde_json::from_str(&job.checklist_json).unwrap_or_default();
                                        rsx! {
                                            div {
                                                key: "{job.id}",
                                                class: "rounded-xl border border-border/40 bg-background/60 p-5 space-y-3 transition-all hover:bg-secondary/15 hover:shadow-md flex flex-col gap-2",
                                                div { class: "flex items-center justify-between gap-2 border-b border-border/30 pb-2",
                                                    div { class: "flex items-center gap-2",
                                                        components::LucideIcon { name: "wrench", class: "h-4 w-4 text-primary" }
                                                        h4 { class: "text-sm font-bold text-foreground m-0", "{job.title}" }
                                                    }
                                                }
                                                div { class: "space-y-1.5 text-xs flex flex-col gap-1.5",
                                                    if checklist_items.is_empty() {
                                                        div { class: "text-muted-foreground", "Inga checklist-punkter för detta uppdrag." }
                                                    } else {
                                                        for item in checklist_items.iter() {
                                                            div { class: "flex items-center gap-2",
                                                                components::LucideIcon {
                                                                    name: if item.done { "check" } else { "circle" },
                                                                    class: if item.done { "h-3.5 w-3.5 text-emerald-500" } else { "h-3.5 w-3.5 text-muted-foreground/75" }
                                                                }
                                                                span {
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
                    }
                }
            } else {
                components::Card {
                    class: "border border-border bg-sidebar shadow-md",
                    components::CardContent {
                        class: "pt-6 space-y-6 flex flex-col gap-4",
                        div { class: "border-b border-border/30 pb-4",
                            h3 { class: "text-base font-bold text-foreground m-0", "Kundsupport & Kontaktinformation" }
                            p { class: "text-xs text-muted-foreground mt-1 m-0", "Vi finns här för att hjälpa dig vardagar 08:00 - 17:00." }
                        }
                        div { class: "text-sm text-foreground leading-relaxed whitespace-pre-line bg-background/30 p-4 rounded-xl border border-border/20 m-0",
                            "TELEFON & E-POST\nKundtjänst: 08-123 45 67\nE-post: support@operations.yntra.se\n\nAVVIKELSER & FELANMÄLAN\nOm du upptäcker fel eller avvikelser i utförandet av uppdraget, vänligen kontakta vår support direkt eller använd meddelandefunktionen i sidomenyn."
                        }
                    }
                }
            }
        }
    }
}
