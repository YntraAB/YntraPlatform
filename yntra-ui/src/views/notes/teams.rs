use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{DailyNote, Team};

#[derive(Props, Clone)]
pub struct TeamOverviewProps {
    pub teams: Vec<Team>,
    pub notes: Vec<DailyNote>,
    pub selected_note_team_id: Signal<String>,
    pub active_note_id: Signal<Option<String>>,
    pub is_composing: Signal<bool>,
    pub edit_mode: Signal<bool>,
    pub search_query: Signal<String>,
    pub locale: String,
}

impl PartialEq for TeamOverviewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TeamOverview(props: TeamOverviewProps) -> Element {
    let teams = props.teams;
    let notes = props.notes;
    let mut selected_note_team_id = props.selected_note_team_id;
    let mut active_note_id = props.active_note_id;
    let mut is_composing = props.is_composing;
    let mut edit_mode = props.edit_mode;
    let mut search_query = props.search_query;
    let locale = props.locale;

    let filtered_teams: Vec<Team> = if search_query.read().is_empty() {
        teams.clone()
    } else {
        let q = search_query.read().to_lowercase();
        teams
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&q))
            .cloned()
            .collect()
    };

    let mut notes_count_by_team = std::collections::HashMap::new();
    let mut latest_note_by_team = std::collections::HashMap::new();
    for note in notes.iter() {
        *notes_count_by_team.entry(&note.team_id).or_insert(0_usize) += 1;
        if let Some(existing) = latest_note_by_team.get(&note.team_id) {
            let existing: &&DailyNote = existing;
            if note.created_at > existing.created_at {
                latest_note_by_team.insert(&note.team_id, note);
            }
        } else {
            latest_note_by_team.insert(&note.team_id, note);
        }
    }

    rsx! {
        div {
            class: "flex flex-col h-full w-full bg-background box-border",

            // Header bar matching reference TeamOverview
            div {
                class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-white/[0.02] box-border backdrop-blur-md",
                h2 { class: "text-lg font-bold text-foreground m-0", "{t(\"notes-teams-title\", &locale)}" }
                div { class: "flex items-center gap-4",
                    div {
                        class: "flex items-center relative w-64",
                        components::LucideIcon { name: "directory", class: "absolute left-3 h-4 w-4", color: "var(--text-muted)" }
                        input {
                            class: "yntra-input pl-9 text-xs h-8",

                            placeholder: "{t(\"notes-teams-search-placeholder\", &locale)}",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                        }
                    }
                }
            }

            // Scrollable teams list
            div {
                class: "scrollbar-dark flex-1 overflow-y-auto w-full box-border",
                if filtered_teams.is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center text-muted-foreground/60 gap-4 py-20",
                        components::LucideIcon { name: "directory", size: "48", color: "var(--text-muted)", class: "opacity-20", }
                        p { class: "text-sm m-0", "{t(\"notes-teams-empty-state\", &locale)}" }
                    }
                } else {
                    div {
                        class: "flex flex-col w-full",
                        for team in filtered_teams.iter() {
                            {
                                let team_id = team.id.clone();
                                let team_name = team.name.clone();

                                // Count notes in this team
                                let notes_count = *notes_count_by_team.get(&team_id).unwrap_or(&0);

                                // Find latest updated note timestamp
                                let latest_note = latest_note_by_team.get(&team_id).copied();

                                rsx! {
                                    div {
                                        key: "{team_id}",
                                        onclick: move |_| {
                                            selected_note_team_id.set(team_id.clone());
                                            active_note_id.set(None);
                                            is_composing.set(false);
                                            edit_mode.set(false);
                                        },
                                        class: "flex flex-row items-center justify-between border-b border-border px-8 py-4 cursor-pointer bg-white/[0.01] list-item-hover transition-colors duration-155",

                                        // Left icon and details
                                        div {
                                            class: "flex items-center gap-4 flex-1 min-w-0",
                                            div {
                                                class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-white/[0.04] text-primary",
                                                components::LucideIcon { name: "notes", class: "h-5 w-5", }
                                            }
                                            div {
                                                class: "flex flex-col gap-1 min-w-0",
                                                span { class: "font-semibold text-sm text-foreground", "{team_name}" }
                                                span { class: "text-xs text-muted-foreground/60 uppercase tracking-wide",
                                                    "{notes_count} {t(\"notes-teams-notes-count\", &locale)}"
                                                }
                                            }
                                        }

                                        // Last updated note date
                                        div {
                                            class: "flex items-center gap-4",
                                            if let Some(note) = latest_note {
                                                {
                                                    let date_str = if note.created_at.len() >= 16 { note.created_at[..16].to_string() } else { note.created_at.clone() };
                                                    rsx! {
                                                        div {
                                                            class: "text-right text-xs text-muted-foreground hidden md:block",
                                                            div { class: "text-[11px] font-bold uppercase tracking-wide text-muted-foreground/60", "{t(\"notes-teams-last-updated\", &locale)}" }
                                                            div { class: "mt-0.5", "{date_str}" }
                                                        }
                                                    }
                                                }
                                            }
                                            components::LucideIcon { name: "chevron-right", class: "h-5 w-5", color: "var(--text-muted)" }
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
