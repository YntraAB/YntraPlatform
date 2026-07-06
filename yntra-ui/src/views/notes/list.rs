use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, DailyNote};
use crate::locales::t;
use crate::components;

#[derive(Props, Clone)]
pub struct NoteListProps {
    pub users: Vec<WorkspaceUser>,
    pub filtered_notes: Vec<DailyNote>,
    pub team_name: String,
    pub selected_note_team_id: Signal<String>,
    pub active_note_id: Signal<Option<String>>,
    pub is_composing: Signal<bool>,
    pub edit_mode: Signal<bool>,
    pub note_subject: Signal<String>,
    pub note_content: Signal<String>,
    pub note_search_query: Signal<String>,
    pub locale: String,
}

impl PartialEq for NoteListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn NoteList(props: NoteListProps) -> Element {
    let users = props.users;
    let filtered_notes = props.filtered_notes;
    let team_name = props.team_name;
    let mut selected_note_team_id = props.selected_note_team_id;
    let mut active_note_id = props.active_note_id;
    let mut is_composing = props.is_composing;
    let mut edit_mode = props.edit_mode;
    let mut note_subject = props.note_subject;
    let mut note_content = props.note_content;
    let mut note_search_query = props.note_search_query;
    let locale = props.locale;

    rsx! {
        div {
            class: "flex flex-col h-full w-full bg-background relative box-border",
            
            // Header bar matching reference NoteList
            div {
                class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-white/[0.02] box-border backdrop-blur-md",
                div {
                    class: "flex items-center gap-3",
                    button {
                        class: "yntra-btn secondary flex items-center justify-center bg-transparent border-0 rounded-full text-muted-foreground cursor-pointer p-1.5 hover:text-foreground",
                        onclick: move |_| {
                            selected_note_team_id.set(String::new());
                            active_note_id.set(None);
                        },
                        components::LucideIcon { name: "chevron-left", size: "20" }
                    }
                    h2 { class: "text-lg font-bold text-foreground m-0", 
                        "{team_name} {t(\"notes-list-title-suffix\", &locale)}"
                    }
                }
                div { class: "flex items-center gap-4",
                    div {
                        class: "flex items-center relative w-64",
                        components::LucideIcon { name: "directory", class: "absolute left-3 h-4 w-4", color: "var(--text-muted)" }
                        input {
                            class: "yntra-input pl-9 text-xs h-8",
                            
                            placeholder: "{t(\"notes-list-search-placeholder\", &locale)}",
                            value: "{note_search_query}",
                            oninput: move |e| note_search_query.set(e.value()),
                        }
                    }
                }
            }

            // Scrollable notes list
            div {
                class: "scrollbar-dark flex-1 overflow-y-auto w-full box-border",
                if filtered_notes.is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center text-muted-foreground/60 gap-4 py-20",
                        components::LucideIcon { name: "notes", size: "48", color: "var(--text-muted)", class: "opacity-20", }
                        p { class: "text-sm m-0", "{t(\"notes-list-empty-state\", &locale)}" }
                    }
                } else {
                    {
                        let buffer_sig = use_signal(|| 5_usize);
                        let notes = filtered_notes.clone();
                        let users_list = users.clone();
                        rsx! {
                            components::VirtualList {
                                count: notes.len(),
                                buffer: buffer_sig,
                                estimate_size: move |_| 60_u32,
                                render_item: move |idx: usize| {
                                    let note = &notes[idx];
                                    let author = users_list
                                        .iter()
                                        .find(|u| Some(u.id.clone()) == note.author_id)
                                        .and_then(|u| u.full_name.clone())
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    let note_id = note.id.clone();
                                    let note_subj = note.subject.clone();
                                    let note_content_snippet = if note.content.len() > 60 { format!("{}...", &note.content[..60]) } else { note.content.clone() };
                                    let date_str = if note.created_at.len() >= 10 { note.created_at[..10].to_string() } else { note.created_at.clone() };

                                    rsx! {
                                        div {
                                            key: "{note_id}",
                                            onclick: move |_| {
                                                active_note_id.set(Some(note_id.clone()));
                                                is_composing.set(false);
                                                edit_mode.set(false);
                                            },
                                            class: "flex flex-row items-center justify-between border-b border-border px-8 py-5 cursor-pointer bg-white/[0.01] list-item-hover transition-colors duration-150 text-sm",
                                            
                                            // Author column
                                            div {
                                                class: "shrink-0 font-semibold text-foreground w-40 truncate pr-4 box-border",
                                                "{author}"
                                            }

                                            // Subject and snippet content
                                            div {
                                                class: "flex-1 flex items-center gap-2 min-w-0 truncate pr-4 box-border",
                                                span { class: "font-semibold text-foreground", "{note_subj}" }
                                                span { class: "text-muted-foreground/60", "- {note_content_snippet}" }
                                            }

                                            // Date column
                                            div {
                                                class: "shrink-0 text-right text-muted-foreground/60 text-xs w-32",
                                                "{date_str}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Floating action button (FAB) for composing new note
            div {
                class: "absolute bottom-8 right-8 z-10",
                button {
                    class: "yntra-btn flex h-12 items-center gap-2 rounded-full bg-white pl-5 pr-6 font-medium text-black shadow-xl shadow-black/50 transition-transform hover:scale-105 hover:bg-neutral-200 cursor-pointer border-0 text-sm",
                    onclick: move |_| {
                        note_subject.set(String::new());
                        note_content.set(String::new());
                        is_composing.set(true);
                    },
                    components::LucideIcon { name: "notes", class: "h-5 w-5", color: "var(--bg-main)" }
                    "{t(\"notes-list-new-note-button\", &locale)}"
                }
            }
        }
    }
}
