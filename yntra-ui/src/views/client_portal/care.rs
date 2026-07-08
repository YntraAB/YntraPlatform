use dioxus::prelude::*;
use yntra_core::{TeamEvent, WorkspaceUser, get_journals, get_medications};
use crate::components;
use crate::state::AppState;

#[derive(Props, Clone)]
pub struct CarePortalProps {
    pub client_id: String,
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub todays_events: Vec<TeamEvent>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for CarePortalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CarePortal(props: CarePortalProps) -> Element {
    let client_id = props.client_id;
    let active_user = props.active_user;
    let users = props.users.clone();
    let todays_events = props.todays_events.clone();
    let _db_trigger = props.db_trigger;
    let state = use_context::<AppState>();

    let mut active_tab = use_signal(|| "medications".to_string());

    let user_names: std::collections::HashMap<String, String> = users
        .iter()
        .map(|u| (u.id.clone(), u.full_name.clone().unwrap_or_default()))
        .collect();

    // Fetch resources locally
    let client_id_for_meds = client_id.clone();
    let actor_id_for_meds = active_user.id.clone();
    let meds_res = use_resource(move || {
        let _trig = state.trigger_clients.read();
        let cid = client_id_for_meds.clone();
        let aid = actor_id_for_meds.clone();
        async move { get_medications(cid, aid).await.unwrap_or_default() }
    });

    let client_id_for_journals = client_id.clone();
    let actor_id_for_journals = active_user.id.clone();
    let journals_res = use_resource(move || {
        let _trig = state.trigger_clients.read();
        let cid = client_id_for_journals.clone();
        let aid = actor_id_for_journals.clone();
        async move { get_journals(cid, aid).await.unwrap_or_default() }
    });

    let client_meds = meds_res.read().clone().unwrap_or_default();
    let client_journals = journals_res.read().clone().unwrap_or_default();

    rsx! {
        // 4. Care & Assistance Portal Widgets (Default)
        div { class: "grid grid-cols-1 gap-6 md:grid-cols-3",
            
            // Today's Schedule Card (takes col-span-2)
            div { class: "md:col-span-2",
                components::Card {
                    class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                    components::CardHeader {
                        class: "pb-3 border-b border-border/40",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                components::LucideIcon { name: "scheduling", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold", "Dagens Assistans" }
                                components::CardDescription { class: "text-xs", "Planerade besök och insatser för idag." }
                            }
                        }
                    }
                    components::CardContent {
                        class: "pt-4 flex flex-1 flex-col justify-between",
                        if todays_events.is_empty() {
                            div { class: "flex flex-col items-center justify-center py-8 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/20 my-auto",
                                components::LucideIcon { name: "clock", class: "h-8 w-8 opacity-20 mb-2" }
                                p { class: "text-sm m-0", "Inga planerade besök idag." }
                            }
                        } else {
                            div { class: "space-y-4",
                                for (ev, start_time_part, end_time_part, caregiver) in todays_events.iter().map(|ev| {
                                    let start_time_part = if ev.start_time.len() >= 16 { ev.start_time[11..16].to_string() } else { ev.start_time.clone() };
                                    let end_time_part = if ev.end_time.len() >= 16 { ev.end_time[11..16].to_string() } else { ev.end_time.clone() };
                                    let caregiver = ev.assignee_id
                                        .as_ref()
                                        .and_then(|aid| user_names.get(aid).cloned())
                                        .filter(|name| !name.is_empty())
                                        .unwrap_or_else(|| "Okänd personal".to_string());
                                    (ev, start_time_part, end_time_part, caregiver)
                                }) {
                                    div {
                                        key: "{ev.id}",
                                        class: "flex items-start gap-4 rounded-xl border border-border/30 bg-background/50 p-4 transition-all hover:bg-secondary/20",
                                        div { class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary font-bold",
                                            components::LucideIcon { name: "directory", class: "h-4 w-4" }
                                        }
                                        div { class: "flex-1 min-w-0",
                                            h4 { class: "text-sm font-bold text-foreground truncate m-0", "{ev.title}" }
                                            p { class: "text-xs text-muted-foreground mt-0.5 line-clamp-1 m-0", "Personal: {caregiver}" }
                                            div { class: "flex items-center gap-1.5 text-xs text-primary font-semibold mt-2",
                                                components::LucideIcon { name: "clock", class: "h-3 w-3" }
                                                span { "{start_time_part} - {end_time_part}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Quick Stats Card (takes col-span-1)
            div { class: "col-span-1",
                components::Card {
                    class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                    components::CardHeader {
                        class: "pb-3 border-b border-border/40",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                components::LucideIcon { name: "heart", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold", "Hälsoöversikt" }
                                components::CardDescription { class: "text-xs", "Kortfattad status för din care plan" }
                            }
                        }
                    }
                    components::CardContent {
                        class: "pt-6 space-y-4 flex-1 flex flex-col justify-center",
                        div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                            span { class: "text-xs font-semibold text-muted-foreground", "Mediciner" }
                            span { class: "text-sm font-bold text-foreground", "{client_meds.len()} st" }
                        }
                        div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                            span { class: "text-xs font-semibold text-muted-foreground", "Journalanteckningar" }
                            span { class: "text-sm font-bold text-foreground", "{client_journals.len()} st" }
                        }
                        div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                            span { class: "text-xs font-semibold text-muted-foreground", "Besök idag" }
                            span { class: "text-sm font-bold text-primary", "{todays_events.len()} st" }
                        }
                    }
                }
            }
        }

        // Expanded Tabs Section (Care)
        div { class: "w-full animate-in fade-in zoom-in duration-300",
            div { class: "grid w-full max-w-md grid-cols-2 bg-muted/50 rounded-xl p-1 mb-6 border border-border",
                button {
                    onclick: move |_| active_tab.set("medications".to_string()),
                    class: if *active_tab.read() == "medications" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "pill", class: "h-3.5 w-3.5" }
                    "Mina Mediciner"
                }
                button {
                    onclick: move |_| active_tab.set("journals".to_string()),
                    class: if *active_tab.read() == "journals" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "book-open", class: "h-3.5 w-3.5" }
                    "Min Journal logg"
                }
            }

            if *active_tab.read() == "medications" {
                components::Card {
                    class: "border border-border bg-sidebar shadow-md",
                    components::CardContent {
                        class: "pt-6",
                        if client_meds.is_empty() {
                            div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10",
                                components::LucideIcon { name: "pill", class: "h-8 w-8 opacity-20 mb-2" }
                                p { class: "text-sm font-semibold m-0", "Inga listade mediciner." }
                            }
                        } else {
                            div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                                for med in client_meds.iter() {
                                    div {
                                        key: "{med.id}",
                                        class: "rounded-xl border border-border/40 bg-background/60 p-5 space-y-3 transition-all hover:bg-secondary/15 hover:shadow-md flex flex-col gap-2",
                                        div { class: "flex items-center justify-between gap-2 border-b border-border/30 pb-2",
                                            div { class: "flex items-center gap-2",
                                                components::LucideIcon { name: "pill", class: "h-4 w-4 text-primary" }
                                                h4 { class: "text-sm font-bold text-foreground m-0", "{med.name}" }
                                            }
                                            if let Some(ref dosage) = med.dosage {
                                                span { class: "rounded-full bg-primary/10 px-2.5 py-0.5 text-[10px] font-bold text-primary",
                                                    "{dosage}"
                                                }
                                            }
                                        }
                                        div { class: "space-y-1.5 text-xs flex flex-col gap-1",
                                            if let Some(ref freq) = med.frequency {
                                                div { class: "flex items-center gap-2",
                                                    components::LucideIcon { name: "clock", class: "h-3.5 w-3.5 text-muted-foreground/75" }
                                                    span { class: "text-muted-foreground",
                                                        strong { class: "text-foreground", "Frekvens: " }
                                                        "{freq}"
                                                    }
                                                }
                                            }
                                            if let Some(ref inst) = med.instructions {
                                                div { class: "flex items-start gap-2 pt-1 border-t border-border/10",
                                                    components::LucideIcon { name: "alert-circle", class: "h-3.5 w-3.5 text-muted-foreground/75 mt-0.5" }
                                                    span { class: "text-muted-foreground leading-relaxed",
                                                        strong { class: "text-foreground", "Instruktioner: " }
                                                        "{inst}"
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
                        class: "pt-6",
                        if client_journals.is_empty() {
                            div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10",
                                components::LucideIcon { name: "book-open", class: "h-8 w-8 opacity-20 mb-2" }
                                p { class: "text-sm font-semibold m-0", "Inga journalanteckningar tillgängliga." }
                            }
                        } else {
                            div { class: "relative border-l border-border/40 pl-6 ml-4 space-y-6 flex flex-col gap-4",
                                for (entry, author_name) in client_journals.iter().map(|entry| {
                                    let author_name = entry.author_id
                                        .as_ref()
                                        .and_then(|aid| user_names.get(aid).cloned())
                                        .filter(|name| !name.is_empty())
                                        .unwrap_or_else(|| "Okänd vårdgivare".to_string());
                                    (entry, author_name)
                                }) {
                                    div {
                                        key: "{entry.id}",
                                        class: "relative space-y-2 flex flex-col gap-1",
                                        // Timeline dot
                                        span { class: "absolute -left-[31px] top-1.5 flex h-[18px] w-[18px] items-center justify-center rounded-full border-2 border-primary bg-background shadow-sm",
                                            span { class: "h-1.5 w-1.5 rounded-full bg-primary" }
                                        }
                                        div { class: "flex flex-col gap-1 border-b border-border/30 pb-3",
                                            div { class: "flex items-center gap-2",
                                                span { class: "text-xs font-bold text-primary",
                                                    "{entry.created_at}"
                                                }
                                                span { class: "text-[10px] bg-secondary/80 px-2 py-0.5 rounded text-muted-foreground",
                                                    "Skrivet av: {author_name}"
                                                }
                                            }
                                            p { class: "text-sm text-foreground leading-relaxed whitespace-pre-line mt-1 bg-background/30 p-4 rounded-xl border border-border/20 m-0",
                                                "{entry.content}"
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
