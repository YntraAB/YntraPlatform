use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, update_note};
use crate::locales::t;
use crate::components;

#[derive(Props, Clone)]
pub struct NoteEditProps {
    pub active_user: WorkspaceUser,
    pub note_id: String,
    pub edit_subject: Signal<String>,
    pub edit_content: Signal<String>,
    pub edit_mode: Signal<bool>,
    pub locale: String,
}

impl PartialEq for NoteEditProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn NoteEdit(props: NoteEditProps) -> Element {
    let active_user = props.active_user;
    let note_id = props.note_id;
    let mut edit_subject = props.edit_subject;
    let mut edit_content = props.edit_content;
    let mut edit_mode = props.edit_mode;
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
                        onclick: move |_| edit_mode.set(false),
                        components::LucideIcon { name: "chevron-left", size: "20" }
                    }
                    h2 { class: "text-lg font-bold text-foreground m-0", 
                        "{t(\"notes-read-title\", &locale)}"
                    }
                }
                div { class: "flex items-center gap-3",
                    button {
                        class: "yntra-btn secondary bg-transparent border border-border text-muted-foreground rounded-lg font-semibold cursor-pointer px-4 py-2",
                        onclick: move |_| edit_mode.set(false),
                        "{t(\"notes-compose-cancel\", &locale)}"
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
                                    let user_id = active_user.id.clone();
                                    spawn(async move {
                                        let _ = update_note(user_id, note_id, author, final_sub, content).await;
                                    });
                                    edit_mode.set(false);
                                }
                            }
                        },
                        "{t(\"common-save\", &locale)}"
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
                        "{t(\"notes-compose-subject-label\", &locale)}"
                    }
                    input {
                        class: "border-none bg-transparent text-foreground text-lg font-medium w-full focus:outline-none focus:ring-0 py-1 px-0 placeholder:text-muted-foreground/50",
                        placeholder: "{t(\"notes-compose-subject-placeholder\", &locale)}",
                        value: "{edit_subject}",
                        oninput: move |e| edit_subject.set(e.value()),
                    }
                }

                // Content textarea
                div {
                    class: "flex flex-col flex-1 pb-10",
                    textarea {
                        class: "border-none bg-transparent text-foreground text-[15px] leading-relaxed w-full flex-1 resize-none focus:outline-none min-h-[300px] p-0 placeholder:text-muted-foreground/50",
                        placeholder: "{t(\"notes-compose-content-placeholder\", &locale)}",
                        value: "{edit_content}",
                        oninput: move |e| edit_content.set(e.value()),
                    }
                }
            }
        }
    }
}
