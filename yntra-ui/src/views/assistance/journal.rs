use crate::components;
use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, ClientProfile, JournalEntry, add_journal_entry};

#[derive(Props, Clone, PartialEq)]
pub struct JournalTabContentProps {
    pub active_user: WorkspaceUser,
    pub current_client: ClientProfile,
    pub client_journals: Vec<JournalEntry>,
    pub users: Vec<WorkspaceUser>,
    pub journal_content: Signal<String>,
    pub is_client: bool,
    pub locale: String,
}

#[component]
pub fn JournalTabContent(props: JournalTabContentProps) -> Element {
    let active_user = props.active_user;
    let current_client = props.current_client;
    let client_journals = props.client_journals;
    let users = props.users;
    let journal_content = props.journal_content;
    let is_client = props.is_client;
    let locale = props.locale;

    let locale_ref = &locale;
    let t_previous_notes = crate::locales::t("assistance-journal-previous-notes", locale_ref);
    let t_journal_no_notes = crate::locales::t("assistance-journal-no-notes", locale_ref);
    let cid_for_submit = current_client.id.clone();
    let author_id = active_user.id.clone();

    rsx! {
        div { class: "space-y-6 flex flex-col gap-6",
            
            // New note form (Caregivers only)
            if !is_client {
                JournalEntryForm {
                    active_user: active_user.clone(),
                    client_id: cid_for_submit.clone(),
                    author_id: author_id.clone(),
                    journal_content,
                    locale: locale.clone(),
                }
            }

            // Previous notes list
            div { class: "space-y-4 flex flex-col gap-4",
                h3 { class: "flex items-center gap-2 text-lg font-bold text-foreground m-0",
                    "{t_previous_notes}"
                    span { class: "inline-flex h-6 min-w-6 items-center justify-center rounded-full bg-muted px-2 text-xs font-medium text-muted-foreground",
                        "{client_journals.len()}"
                    }
                }
                if client_journals.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-border bg-muted/30 py-12 text-center transition-all hover:bg-muted/50",
                        p { class: "text-sm font-medium text-muted-foreground m-0", "{t_journal_no_notes}" }
                    }
                } else {
                    div { class: "grid gap-4",
                        for (entry, author_name) in client_journals.iter().map(|entry| {
                            let author_name = users
                                .iter()
                                .find(|u| Some(u.id.clone()) == entry.author_id)
                                .and_then(|u| u.full_name.clone())
                                .unwrap_or_else(|| crate::locales::t("assistance-journal-unknown-author", locale_ref));
                            (entry, author_name)
                        }) {
                            div {
                                key: "{entry.id}",
                                class: "group relative overflow-hidden rounded-2xl border border-border bg-card p-5 transition-all hover:border-primary/20 hover:shadow-md flex flex-col gap-3",
                                div { class: "mb-3 flex items-center justify-between",
                                    div { class: "flex items-center gap-3",
                                        div { class: "h-8 w-8 rounded-full bg-primary/10 p-1 flex items-center justify-center",
                                            div { class: "h-full w-full rounded-full bg-primary/25" }
                                        }
                                        div {
                                            span { class: "block text-sm font-bold text-foreground", "{author_name}" }
                                            span { class: "text-[10px] font-medium uppercase tracking-tighter text-muted-foreground",
                                                "{entry.created_at}"
                                            }
                                        }
                                    }
                                }
                                div { class: "relative pl-3",
                                    div { class: "absolute left-0 top-0 h-full w-1 rounded-full bg-primary/10 transition-all group-hover:bg-primary/30" }
                                    p { class: "whitespace-pre-wrap text-sm leading-relaxed text-foreground/90 m-0",
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

#[derive(Props, Clone, PartialEq)]
struct JournalEntryFormProps {
    active_user: WorkspaceUser,
    client_id: String,
    author_id: String,
    journal_content: Signal<String>,
    locale: String,
}

#[component]
fn JournalEntryForm(props: JournalEntryFormProps) -> Element {
    let active_user = props.active_user;
    let client_id = props.client_id;
    let author_id = props.author_id;
    let mut journal_content = props.journal_content;
    let locale_ref = &props.locale;
    let t_write_new_note = crate::locales::t("assistance-journal-write-new-note", locale_ref);
    let t_placeholder = crate::locales::t("assistance-journal-placeholder", locale_ref);
    let t_save_note = crate::locales::t("assistance-journal-save-note", locale_ref);

    rsx! {
        div {
            class: "group relative overflow-hidden rounded-2xl border border-border bg-card p-5 transition-all hover:shadow-lg flex flex-col gap-3",
            div { class: "absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent opacity-50 pointer-events-none" }
            h3 { class: "relative m-0 flex items-center gap-2 text-sm font-semibold uppercase tracking-wider text-muted-foreground",
                components::LucideIcon { name: "messaging", class: "h-4 w-4 text-primary" }
                "{t_write_new_note}"
            }
            textarea {
                value: "{journal_content}",
                oninput: move |e| journal_content.set(e.value()),
                class: "relative min-h-[120px] w-full resize-none rounded-xl border border-border bg-background/50 p-4 text-sm transition-all focus:border-primary/50 focus:outline-none focus:ring-4 focus:ring-primary/10",
                placeholder: "{t_placeholder}",
            }
            div { class: "relative flex justify-end",
                button {
                    disabled: journal_content.read().trim().is_empty(),
                    onclick: move |_| {
                        let text = journal_content.read().trim().to_string();
                        if !text.is_empty() {
                            let workspace_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                            let cid = client_id.clone();
                            let aid = author_id.clone();
                            spawn(async move {
                                let _ = add_journal_entry(
                                    workspace_id,
                                    cid,
                                    aid,
                                    text,
                                ).await;
                            });
                            journal_content.set(String::new());
                        }
                    },
                    class: "rounded-full bg-primary hover:bg-primary/95 text-primary-foreground font-semibold px-6 py-2 cursor-pointer transition-all border-0 shadow-sm disabled:opacity-50",
                    "{t_save_note}"
                }
            }
        }
    }
}
