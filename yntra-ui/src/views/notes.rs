use dioxus::prelude::*;
use yntra_core::DailyNote;
use yntra_core::Team;
use yntra_core::WorkspaceUser;
use yntra_core::{add_note, update_note, delete_note};
use crate::locales::t;
use crate::components;

#[derive(Props, Clone)]
pub struct NotesViewProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub teams: Vec<Team>,
    pub notes: Vec<DailyNote>,
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

#[derive(Debug, Clone, PartialEq)]
enum DiffType {
    Added,
    Removed,
    Unchanged,
}

struct DiffSegment {
    r#type: DiffType,
    text: String,
}

fn get_diff_segments(old_str: &str, new_str: &str) -> Vec<DiffSegment> {
    let old_words: Vec<String> = old_str.split_whitespace().map(|s| s.to_string()).collect();
    let new_words: Vec<String> = new_str.split_whitespace().map(|s| s.to_string()).collect();

    let mut matrix = vec![vec![0; new_words.len() + 1]; old_words.len() + 1];

    for i in 1..=old_words.len() {
        for j in 1..=new_words.len() {
            if old_words[i - 1] == new_words[j - 1] {
                matrix[i][j] = matrix[i - 1][j - 1] + 1;
            } else {
                matrix[i][j] = std::cmp::max(matrix[i - 1][j], matrix[i][j - 1]);
            }
        }
    }

    let mut segments = Vec::new();
    let mut i = old_words.len();
    let mut j = new_words.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_words[i - 1] == new_words[j - 1] {
            segments.insert(0, DiffSegment {
                r#type: DiffType::Unchanged,
                text: format!("{} ", old_words[i - 1]),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || matrix[i][j - 1] >= matrix[i - 1][j]) {
            segments.insert(0, DiffSegment {
                r#type: DiffType::Added,
                text: format!("{} ", new_words[j - 1]),
            });
            j -= 1;
        } else if i > 0 && (j == 0 || matrix[i][j - 1] < matrix[i - 1][j]) {
            segments.insert(0, DiffSegment {
                r#type: DiffType::Removed,
                text: format!("{} ", old_words[i - 1]),
            });
            i -= 1;
        }
    }

    segments
}

#[component]
pub fn NotesView(props: NotesViewProps) -> Element {
    let active_user = props.active_user;
    let users = props.users.clone();
    let teams = props.teams.clone();
    let notes = props.notes.clone();

    let mut selected_note_team_id = props.selected_note_team_id;
    let mut note_subject = props.note_subject;
    let mut note_content = props.note_content;

    let mut active_note_id = props.active_note_id;
    let mut is_composing = props.is_composing;
    let mut edit_mode = use_signal(|| false);
    let mut search_query = use_signal(String::new);
    let mut note_search_query = use_signal(String::new);

    let mut edit_subject = use_signal(String::new);
    let mut edit_content = use_signal(String::new);
    let mut expanded_note_history_id = use_signal(|| Option::<String>::None);

    let active_team_id = selected_note_team_id.read().clone();

    // 1. Render TeamOverview if no team is selected
    if active_team_id.is_empty() {
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

        rsx! {
            div {
                class: "flex flex-col h-full w-full bg-background box-border",
                
                // Header bar matching reference TeamOverview
                div {
                    class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-white/[0.02] box-border backdrop-blur-md",
                    h2 { class: "text-lg font-bold text-foreground m-0", "{t(\"notes-teams-title\", &props.locale)}" }
                    div { class: "flex items-center gap-4",
                        div {
                            class: "flex items-center relative w-64",
                            components::LucideIcon { name: "directory", class: "absolute left-3 h-4 w-4", color: "var(--text-muted)" }
                            input {
                                class: "yntra-input pl-9 text-xs h-8",
                                
                                placeholder: "{t(\"notes-teams-search-placeholder\", &props.locale)}",
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
                            p { class: "text-sm m-0", "{t(\"notes-teams-empty-state\", &props.locale)}" }
                        }
                    } else {
                        div {
                            class: "flex flex-col w-full",
                            for team in filtered_teams.iter() {
                                {
                                    let team_id = team.id.clone();
                                    let team_name = team.name.clone();
                                    
                                    // Count notes in this team
                                    let notes_count = notes.iter().filter(|n| n.team_id == team_id).count();
                                    
                                    // Find latest updated note timestamp
                                    let latest_note = notes
                                        .iter()
                                        .filter(|n| n.team_id == team_id)
                                        .max_by_key(|n| &n.created_at);

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
                                                        "{notes_count} {t(\"notes-teams-notes-count\", &props.locale)}"
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
                                                                div { class: "text-[11px] font-bold uppercase tracking-wide text-muted-foreground/60", "{t(\"notes-teams-last-updated\", &props.locale)}" }
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
    } else {
        // Load active team details
        let selected_team = teams.iter().find(|t| t.id == active_team_id);
        let team_name = selected_team.map(|t| t.name.clone()).unwrap_or_default();

        // 2. Render NoteComposePane if is_composing is true
        if *is_composing.read() {
            rsx! {
                div {
                    class: "flex flex-col h-full w-full bg-background box-border",
                    
                    // Header bar matching reference NoteComposePane
                    div {
                        class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-white/[0.02] box-border backdrop-blur-md",
                        div {
                            class: "flex items-center gap-3",
                            button {
                                class: "yntra-btn secondary flex items-center justify-center bg-transparent border-0 rounded-full text-muted-foreground cursor-pointer p-1.5 hover:text-foreground",
                                onclick: move |_| is_composing.set(false),
                                components::LucideIcon { name: "chevron-left", size: "20" }
                            }
                            h2 { class: "text-lg font-bold text-foreground m-0", 
                                "{t(\"notes-compose-title\", &props.locale)} {team_name}"
                            }
                        }
                        div { class: "flex items-center gap-3",
                            button {
                                class: "yntra-btn secondary bg-transparent border border-border text-muted-foreground rounded-lg font-semibold cursor-pointer px-4 py-2",
                                onclick: move |_| is_composing.set(false),
                                "{t(\"notes-compose-cancel\", &props.locale)}"
                            }
                            button {
                                class: "yntra-btn rounded-full font-bold cursor-pointer px-5 py-2",
                                style: "background: var(--accent); color: var(--bg-main);",
                                disabled: note_content.read().trim().is_empty(),
                                onclick: move |_| {
                                    let sub = note_subject.read().trim().to_string();
                                    let content = note_content.read().trim().to_string();
                                    if !content.is_empty() {
                                        let final_sub = if sub.is_empty() { "Untitled Note".to_string() } else { sub };
                                         let workspace_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                         let team_id = active_team_id.clone();
                                         let user_id = active_user.id.clone();
                                         spawn(async move {
                                             let _ = add_note(
                                                 workspace_id,
                                                 team_id,
                                                 user_id,
                                                 final_sub,
                                                 content,
                                             ).await;
                                         });
                                        note_subject.set(String::new());
                                        note_content.set(String::new());
                                        is_composing.set(false);
                                    }
                                },
                                "{t(\"notes-compose-save-button\", &props.locale)}"
                            }
                        }
                    }

                    // Compose content editor area
                    div {
                        class: "scrollbar-dark flex-1 overflow-y-auto px-8 py-10 flex flex-col gap-8 mx-auto w-full max-w-[800px] box-border md:px-24 lg:px-48",
                        
                        // Subject input
                        div {
                            class: "flex flex-col border-b border-border pb-2 transition-colors",
                            label { class: "text-[11px] font-bold uppercase tracking-wider text-muted-foreground mb-1",
                                "{t(\"notes-compose-subject-label\", &props.locale)}"
                            }
                            input {
                                class: "border-none bg-transparent text-foreground text-lg font-medium w-full focus:outline-none focus:ring-0 py-1 px-0 placeholder:text-muted-foreground/50",
                                placeholder: "{t(\"notes-compose-subject-placeholder\", &props.locale)}",
                                value: "{note_subject}",
                                oninput: move |e| note_subject.set(e.value()),
                                autofocus: true,
                            }
                        }

                        // Content textarea
                        div {
                            class: "flex flex-col flex-1 pb-10",
                            textarea {
                                class: "border-none bg-transparent text-foreground text-[15px] leading-relaxed w-full flex-1 resize-none focus:outline-none min-h-[300px] p-0 placeholder:text-muted-foreground/50",
                                placeholder: "{t(\"notes-compose-content-placeholder\", &props.locale)}",
                                value: "{note_content}",
                                oninput: move |e| note_content.set(e.value()),
                            }
                        }
                    }
                }
            }
        } else {
            // 3. Filter notes of selected team
            let team_notes: Vec<DailyNote> = notes
                .iter()
                .filter(|n| n.team_id == active_team_id)
                .cloned()
                .collect();

            let filtered_notes: Vec<DailyNote> = if note_search_query.read().is_empty() {
                team_notes.clone()
            } else {
                let q = note_search_query.read().to_lowercase();
                team_notes
                    .iter()
                    .filter(|n| n.subject.to_lowercase().contains(&q) || n.content.to_lowercase().contains(&q))
                    .cloned()
                    .collect()
            };

            // 4. Render NoteReadPane if active_note_id is Some
            if let Some(target_id) = active_note_id.read().clone() {
                if let Some(note) = notes.iter().find(|n| n.id == target_id) {
                    let author = users
                        .iter()
                        .find(|u| Some(u.id.clone()) == note.author_id)
                        .and_then(|u| u.full_name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let is_author = note.author_id == Some(active_user.id.clone());
                    let is_manager = active_user.role == "platform_admin" || active_user.role == "admin";
                    let can_delete = is_author || is_manager;
                    let note_id = note.id.clone();
                    
                    let is_history_expanded = Some(note.id.clone()) == *expanded_note_history_id.read();

                    // Parse edit history
                    let history_val: serde_json::Value = serde_json::from_str(&note.edit_history)
                        .unwrap_or_else(|_| serde_json::json!([]));
                    let history_arr = history_val.as_array();
                    let history_len = history_arr.map(|a| a.len()).unwrap_or(0);

                    // 4a. NoteEditPane if edit_mode is true
                    if *edit_mode.read() {
                        rsx! {
                            div {
                                class: "flex flex-col h-full w-full bg-background box-border",
                                
                                // Header bar matching reference NoteComposePane
                                div {
                                    class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-white/[0.02] box-border backdrop-blur-md",
                                    div {
                                        class: "flex items-center gap-3",
                                        button {
                                            class: "yntra-btn secondary flex items-center justify-center bg-transparent border-0 rounded-full text-muted-foreground cursor-pointer p-1.5 hover:text-foreground",
                                            onclick: move |_| edit_mode.set(false),
                                            components::LucideIcon { name: "chevron-left", size: "20" }
                                        }
                                        h2 { class: "text-lg font-bold text-foreground m-0", 
                                            "{t(\"notes-read-title\", &props.locale)}"
                                        }
                                    }
                                    div { class: "flex items-center gap-3",
                                        button {
                                            class: "yntra-btn secondary bg-transparent border border-border text-muted-foreground rounded-lg font-semibold cursor-pointer px-4 py-2",
                                            onclick: move |_| edit_mode.set(false),
                                            "{t(\"notes-compose-cancel\", &props.locale)}"
                                        }
                                        button {
                                            class: "yntra-btn rounded-full font-bold cursor-pointer px-5 py-2",
                                            style: "background: var(--accent); color: var(--bg-main);",
                                            disabled: edit_content.read().trim().is_empty(),
                                            onclick: {
                                                let n_id = note_id.clone();
                                                let author_name = active_user.full_name.clone().unwrap_or_else(|| "You".to_string());
                                                move |_| {
                                                    let sub = edit_subject.read().trim().to_string();
                                                    let content = edit_content.read().trim().to_string();
                                                    if !content.is_empty() {
                                                        let final_sub = if sub.is_empty() { "Untitled Note".to_string() } else { sub };
                                                         let note_id = n_id.clone();
                                                         let author = author_name.clone();
                                                         spawn(async move {
                                                             let _ = update_note(note_id, author, final_sub, content).await;
                                                         });
                                                         edit_mode.set(false);
                                                    }
                                                }
                                            },
                                            "{t(\"common-save\", &props.locale)}"
                                        }
                                    }
                                }

                                // Edit content editor area
                                div {
                                    class: "scrollbar-dark flex-1 overflow-y-auto px-8 py-10 flex flex-col gap-8 mx-auto w-full max-w-[800px] box-border md:px-24 lg:px-48",
                                    
                                    // Subject input
                                    div {
                                        class: "flex flex-col border-b border-border pb-2 transition-colors",
                                        label { class: "text-[11px] font-bold uppercase tracking-wider text-muted-foreground mb-1",
                                            "{t(\"notes-compose-subject-label\", &props.locale)}"
                                        }
                                        input {
                                            class: "border-none bg-transparent text-foreground text-lg font-medium w-full focus:outline-none focus:ring-0 py-1 px-0 placeholder:text-muted-foreground/50",
                                            placeholder: "{t(\"notes-compose-subject-placeholder\", &props.locale)}",
                                            value: "{edit_subject}",
                                            oninput: move |e| edit_subject.set(e.value()),
                                        }
                                    }

                                    // Content textarea
                                    div {
                                        class: "flex flex-col flex-1 pb-10",
                                        textarea {
                                            class: "border-none bg-transparent text-foreground text-[15px] leading-relaxed w-full flex-1 resize-none focus:outline-none min-h-[300px] p-0 placeholder:text-muted-foreground/50",
                                            placeholder: "{t(\"notes-compose-content-placeholder\", &props.locale)}",
                                            value: "{edit_content}",
                                            oninput: move |e| edit_content.set(e.value()),
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // 4b. Normal NoteReadPane view
                        let date_str = if note.created_at.len() >= 10 { note.created_at[..10].to_string() } else { note.created_at.clone() };
                        let time_str = if note.created_at.len() >= 19 { note.created_at[11..16].to_string() } else { String::new() };

                        rsx! {
                            div {
                                class: "flex flex-col h-full w-full bg-background box-border",
                                
                                // Header bar matching reference NoteReadPane
                                div {
                                    class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8 bg-white/[0.02] box-border backdrop-blur-md",
                                    div {
                                        class: "flex items-center gap-3",
                                        button {
                                            class: "yntra-btn secondary flex items-center justify-center bg-transparent border-0 rounded-full text-muted-foreground cursor-pointer p-1.5 hover:text-foreground",
                                            onclick: move |_| active_note_id.set(None),
                                            components::LucideIcon { name: "chevron-left", size: "20" }
                                        }
                                        h2 { class: "text-lg font-bold text-foreground m-0", 
                                            "{t(\"notes-read-title\", &props.locale)}"
                                        }
                                    }
                                    div { class: "flex items-center gap-2",
                                        if is_author {
                                            button {
                                                class: "yntra-btn secondary p-1.5 flex items-center justify-center bg-transparent border-0 rounded-full text-muted-foreground cursor-pointer hover:text-foreground",
                                                onclick: {
                                                    let subj = note.subject.clone();
                                                    let cont = note.content.clone();
                                                    move |_| {
                                                        edit_subject.set(subj.clone());
                                                        edit_content.set(cont.clone());
                                                        edit_mode.set(true);
                                                    }
                                                },
                                                span { class: "text-sm mr-0.5", "✏" }
                                            }
                                        }
                                        if can_delete {
                                            button {
                                                class: "yntra-btn secondary p-1.5 flex items-center justify-center bg-transparent border-0 rounded-full cursor-pointer text-rose-500 hover:text-rose-400 opacity-80 hover:opacity-100",
                                                onclick: {
                                                    let n_id = note_id.clone();
                                                    let uid = active_user.id.clone();
                                                     move |_| {
                                                          let note_id = n_id.clone();
                                                          let uid_clone = uid.clone();
                                                          spawn(async move {
                                                              let _ = delete_note(uid_clone, note_id).await;
                                                          });
                                                          active_note_id.set(None);
                                                      }
                                                },
                                                span { class: "text-sm mr-0.5", "🗑" }
                                            }
                                        }
                                    }
                                }

                                // Scrollable content area
                                div {
                                    class: "scrollbar-dark flex-1 overflow-y-auto px-8 py-10 mx-auto w-full max-w-[800px] box-border md:px-24 lg:px-48",
                                    
                                    // Written by and history header card
                                    div {
                                        class: "flex justify-between items-center border-b border-border mb-8 pb-6",
                                        div {
                                            class: "flex items-center gap-3.5",
                                            div {
                                                class: "flex justify-center items-center rounded-full border border-border font-bold text-foreground h-11 w-11 bg-white/5 text-lg",
                                                span { "{author.chars().next().unwrap_or('?')}" }
                                            }
                                            div {
                                                class: "flex flex-col gap-0.5",
                                                div { class: "text-sm font-semibold text-foreground",
                                                    "{t(\"notes-read-written-by\", &props.locale)} {author}"
                                                }
                                                div { class: "text-xs text-muted-foreground/60",
                                                    "{t(\"notes-read-published\", &props.locale)} {date_str} {t(\"notes-read-at_time\", &props.locale)} {time_str}"
                                                }
                                            }
                                        }

                                        if is_manager && history_len > 0 {
                                            button {
                                                class: "bg-transparent border border-border rounded-lg text-xs font-bold cursor-pointer flex items-center text-amber-500 px-3.5 py-1.5 gap-1.5 transition-colors duration-150",
                                                onclick: move |_| expanded_note_history_id.set(if is_history_expanded { None } else { Some(note_id.clone()) }),
                                                span { "⏳" }
                                                span { "{t(\"notes-read-history-button\", &props.locale)} ({history_len})" }
                                            }
                                        }
                                    }

                                    // Render History Audit Logs inline
                                    if is_history_expanded && history_len > 0 && is_manager {
                                        div {
                                            class: "mb-8 border border-border rounded-lg overflow-hidden bg-black/15 box-border",
                                            div {
                                                class: "flex justify-between items-center border-b border-border text-xs font-bold uppercase tracking-wide text-muted-foreground bg-white/[0.015] px-4 py-3",
                                                div { class: "flex items-center gap-1.5",
                                                    span { "⏳" }
                                                    span { "{t(\"notes-read-audit-log-title\", &props.locale)}" }
                                                }
                                                span { class: "font-medium text-[10px] opacity-70",
                                                    "{history_len} {t(\"notes-read-changes-recorded\", &props.locale)}"
                                                }
                                            }
                                            div {
                                                class: "flex flex-col divide-y divide-border",
                                                if let Some(arr) = history_arr {
                                                    for (idx, entry) in arr.iter().enumerate() {
                                                        {
                                                            let edited_by = entry.get("editedBy").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                                            let edited_at = entry.get("editedAt").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                                            let old_subject = entry.get("oldSubject").and_then(|v| v.as_str());
                                                            let new_subject = entry.get("newSubject").and_then(|v| v.as_str());
                                                            let old_content = entry.get("oldContent").and_then(|v| v.as_str());
                                                            let new_content = entry.get("newContent").and_then(|v| v.as_str());

                                                            rsx! {
                                                                div {
                                                                    key: "{idx}",
                                                                    class: "flex flex-col gap-2 p-4 border-b border-border bg-black/5",
                                                                    div {
                                                                        class: "flex justify-between items-center text-xs text-muted-foreground/60",
                                                                        span { class: "font-semibold", "{t(\"notes-read-edited-by\", &props.locale)}: {edited_by}" }
                                                                        span { "{t(\"common-date\", &props.locale)}: {edited_at}" }
                                                                    }
                                                                    if let (Some(os), Some(ns)) = (old_subject, new_subject) {
                                                                        div {
                                                                            class: "flex items-center gap-1.5 text-xs",
                                                                            span { class: "text-muted-foreground/60", "{t(\"notes-compose-subject-label\", &props.locale)}:" }
                                                                            span { class: "text-rose-500 line-through opacity-60", "{os}" }
                                                                            span { "→" }
                                                                            span { class: "font-bold text-green-500", "{ns}" }
                                                                        }
                                                                    }
                                                                    if let (Some(oc), Some(nc)) = (old_content, new_content) {
                                                                        {
                                                                            let diffs = get_diff_segments(oc, nc);
                                                                            rsx! {
                                                                                div {
                                                                                    class: "flex flex-col gap-1",
                                                                                    span { class: "text-[11px] text-muted-foreground/60 font-semibold", "{t(\"notes-read-diff-label\", &props.locale)}:" }
                                                                                    div {
                                                                                        class: "p-3 text-xs text-muted-foreground bg-black/20 rounded-md font-mono whitespace-pre-wrap leading-relaxed border border-white/[0.02]",
                                                                                        for (s_idx, seg) in diffs.iter().enumerate() {
                                                                                            match seg.r#type {
                                                                                                DiffType::Added => rsx! { span { key: "{s_idx}", class: "bg-emerald-500/25 text-emerald-400 px-0.5 rounded-[2px]", "{seg.text}" } },
                                                                                                DiffType::Removed => rsx! { span { key: "{s_idx}", class: "bg-rose-500/25 text-rose-400 line-through px-0.5 rounded-[2px]", "{seg.text}" } },
                                                                                                DiffType::Unchanged => rsx! { span { key: "{s_idx}", "{seg.text}" } }
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
                                        }
                                    }

                                    // Subject and full content
                                    h3 { class: "font-extrabold text-foreground mt-0 mb-5 text-2xl tracking-tight",
                                        "{note.subject}"
                                    }
                                    div {
                                        class: "text-sm text-muted-foreground m-0 leading-relaxed whitespace-pre-wrap",
                                        "{note.content}"
                                    }
                                 }
                             }
                         }
                     }
                 } else {
                     rsx! { div { "Note not found" } }
                 }
             } else {
                 // 5. Render NoteList if team is selected but no note is open
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
                                     "{team_name} {t(\"notes-list-title-suffix\", &props.locale)}"
                                 }
                             }
                             div { class: "flex items-center gap-4",
                                 div {
                                     class: "flex items-center relative w-64",
                                     components::LucideIcon { name: "directory", class: "absolute left-3 h-4 w-4", color: "var(--text-muted)" }
                                     input {
                                         class: "yntra-input pl-9 text-xs h-8",
                                         
                                         placeholder: "{t(\"notes-list-search-placeholder\", &props.locale)}",
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
                                     p { class: "text-sm m-0", "{t(\"notes-list-empty-state\", &props.locale)}" }
                                 }
                             } else {
                                 div {
                                     class: "flex flex-col w-full",
                                     for note in filtered_notes.iter() {
                                         {
                                             let author = users
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
                                 "{t(\"notes-list-new-note-button\", &props.locale)}"
                             }
                         }
                     }
            }
        }
    }
}
}
