use dioxus::prelude::*;
use yntra_core::{DailyNote, WorkspaceUser};

mod compose;
mod diff;
mod edit;
mod list;
mod read;
mod teams;

pub use compose::NoteCompose;
pub use edit::NoteEdit;
pub use list::NoteList;
pub use read::NoteRead;
pub use teams::TeamOverview;

#[derive(Props, Clone)]
pub struct NotesViewProps {
    pub active_user: WorkspaceUser,
    pub selected_note_team_id: Signal<String>,
    pub note_subject: Signal<String>,
    pub note_content: Signal<String>,
    pub active_note_id: Signal<Option<String>>,
    pub is_composing: Signal<bool>,
    pub locale: String,
}

impl PartialEq for NotesViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn NotesView(props: NotesViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_user = props.active_user;
    let users = state.users.read().clone().unwrap_or_default();
    let teams = state.teams.read().clone().unwrap_or_default();
    let notes = state.notes.read().clone().unwrap_or_default();

    let selected_note_team_id = props.selected_note_team_id;
    let note_subject = props.note_subject;
    let note_content = props.note_content;

    let active_note_id = props.active_note_id;
    let is_composing = props.is_composing;

    let edit_mode = use_signal(|| false);
    let search_query = use_signal(String::new);
    let note_search_query = use_signal(String::new);

    let edit_subject = use_signal(String::new);
    let edit_content = use_signal(String::new);
    let expanded_note_history_id = use_signal(|| Option::<String>::None);

    let active_team_id = selected_note_team_id.read().clone();

    if active_team_id.is_empty() {
        rsx! {
            TeamOverview {
                teams: teams,
                notes: notes,
                selected_note_team_id: selected_note_team_id,
                active_note_id: active_note_id,
                is_composing: is_composing,
                edit_mode: edit_mode,
                search_query: search_query,
                locale: props.locale,
            }
        }
    } else {
        let selected_team = teams.iter().find(|t| t.id == active_team_id);
        let team_name = selected_team.map(|t| t.name.clone()).unwrap_or_default();

        if *is_composing.read() {
            rsx! {
                NoteCompose {
                    active_user: active_user,
                    users: users,
                    active_team_id: active_team_id,
                    team_name: team_name,
                    note_subject: note_subject,
                    note_content: note_content,
                    is_composing: is_composing,
                    locale: props.locale,
                }
            }
        } else {
            let team_notes: Vec<DailyNote> = notes
                .iter()
                .filter(|n| n.team_id == active_team_id)
                .cloned()
                .collect();

            let user_id_s = active_user.id.clone();
            let team_id_s = active_team_id.clone();
            let note_search_res = use_resource(move || {
                let _trig = note_search_query.read();
                let uid = user_id_s.clone();
                let tid = team_id_s.clone();
                let q = note_search_query.read().clone();
                async move {
                    if q.is_empty() {
                        None
                    } else {
                        Some(
                            yntra_core::search_notes(uid, tid, q)
                                .await
                                .unwrap_or_default(),
                        )
                    }
                }
            });

            let filtered_notes: Vec<DailyNote> = if note_search_query.read().is_empty() {
                team_notes.clone()
            } else {
                note_search_res.read().clone().flatten().unwrap_or_default()
            };

            if let Some(target_id) = active_note_id.read().clone() {
                if let Some(note) = notes.iter().find(|n| n.id == target_id) {
                    if *edit_mode.read() {
                        rsx! {
                            NoteEdit {
                                active_user: active_user,
                                users: users,
                                note_id: note.id.clone(),
                                edit_subject: edit_subject,
                                edit_content: edit_content,
                                edit_mode: edit_mode,
                                locale: props.locale,
                            }
                        }
                    } else {
                        rsx! {
                            NoteRead {
                                active_user: active_user,
                                users: users,
                                note: note.clone(),
                                active_note_id: active_note_id,
                                edit_subject: edit_subject,
                                edit_content: edit_content,
                                edit_mode: edit_mode,
                                expanded_note_history_id: expanded_note_history_id,
                                locale: props.locale,
                            }
                        }
                    }
                } else {
                    rsx! { div { "Note not found" } }
                }
            } else {
                rsx! {
                    NoteList {
                        active_user: active_user.clone(),
                        users: users,
                        filtered_notes: filtered_notes,
                        team_name: team_name,
                        selected_note_team_id: selected_note_team_id,
                        active_note_id: active_note_id,
                        is_composing: is_composing,
                        edit_mode: edit_mode,
                        note_subject: note_subject,
                        note_content: note_content,
                        note_search_query: note_search_query,
                        locale: props.locale,
                    }
                }
            }
        }
    }
}
