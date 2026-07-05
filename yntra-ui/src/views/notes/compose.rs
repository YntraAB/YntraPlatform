use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, add_note};
use crate::locales::t;
use crate::components;

#[derive(Props, Clone)]
pub struct NoteComposeProps {
    pub active_user: WorkspaceUser,
    pub active_team_id: String,
    pub team_name: String,
    pub note_subject: Signal<String>,
    pub note_content: Signal<String>,
    pub is_composing: Signal<bool>,
    pub locale: String,
}

impl PartialEq for NoteComposeProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn NoteCompose(props: NoteComposeProps) -> Element {
    let active_user = props.active_user;
    let active_team_id = props.active_team_id;
    let team_name = props.team_name;
    let mut note_subject = props.note_subject;
    let mut note_content = props.note_content;
    let mut is_composing = props.is_composing;
    let locale = props.locale;

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
                        "{t(\"notes-compose-title\", &locale)} {team_name}"
                    }
                }
                div { class: "flex items-center gap-3",
                    button {
                        class: "yntra-btn secondary bg-transparent border border-border text-muted-foreground rounded-lg font-semibold cursor-pointer px-4 py-2",
                        onclick: move |_| is_composing.set(false),
                        "{t(\"notes-compose-cancel\", &locale)}"
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
                        "{t(\"notes-compose-save-button\", &locale)}"
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
                        "{t(\"notes-compose-subject-label\", &locale)}"
                    }
                    input {
                        class: "border-none bg-transparent text-foreground text-lg font-medium w-full focus:outline-none focus:ring-0 py-1 px-0 placeholder:text-muted-foreground/50",
                        placeholder: "{t(\"notes-compose-subject-placeholder\", &locale)}",
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
                        placeholder: "{t(\"notes-compose-content-placeholder\", &locale)}",
                        value: "{note_content}",
                        oninput: move |e| note_content.set(e.value()),
                    }
                }
            }
        }
    }
}
