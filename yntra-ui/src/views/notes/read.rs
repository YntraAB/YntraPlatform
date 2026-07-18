use super::diff::{DiffType, get_diff_segments};
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{DailyNote, WorkspaceUser, delete_note};

#[derive(Props, Clone)]
pub struct NoteReadProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub note: DailyNote,
    pub active_note_id: Signal<Option<String>>,
    pub edit_subject: Signal<String>,
    pub edit_content: Signal<String>,
    pub edit_mode: Signal<bool>,
    pub expanded_note_history_id: Signal<Option<String>>,
    pub locale: String,
}

impl PartialEq for NoteReadProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn NoteRead(props: NoteReadProps) -> Element {
    let active_user = props.active_user;
    let users = props.users;
    let note = props.note;
    let mut active_note_id = props.active_note_id;
    let mut edit_subject = props.edit_subject;
    let mut edit_content = props.edit_content;
    let mut edit_mode = props.edit_mode;
    let mut expanded_note_history_id = props.expanded_note_history_id;
    let locale = props.locale;

    let mut decrypted_content = use_signal(|| Option::<String>::None);
    let state = use_context::<crate::state::AppState>();
    let mut passkey_seed_input = use_signal(move || state.get_passkey_seed());
    let mut decryption_error = use_signal(|| Option::<String>::None);

    let content_str = note.content.clone();
    let parts: Vec<String> = content_str.split(':').map(|s| s.to_string()).collect();
    let (has_enc, proof, ciphertext) = if parts.len() == 3 && parts[0] == "zero_copy_enc" {
        (true, parts[1].clone(), parts[2].clone())
    } else {
        (false, String::new(), String::new())
    };

    let users_clone = users.clone();
    let author_id_clone = note.author_id.clone();
    let ciphertext_clone = ciphertext.clone();
    let proof_verified = use_signal(move || {
        if has_enc {
            let trust = yntra_core::ZkCryptoTrust::new();
            let author_id = author_id_clone.clone().unwrap_or_default();
            let author_user = users_clone.iter().find(|u| u.id == author_id);
            let author_role = author_user
                .map(|u| u.role.clone())
                .unwrap_or_else(|| "user".to_string());
            
            let is_ring = if let Ok(proof_bytes) = const_hex::decode(&proof) {
                proof_bytes.starts_with(b"ZKP_RING_PROOF_V1:")
            } else {
                false
            };

            let public_key_hex = if is_ring {
                let workspace_id = note.workspace_id.clone();
                let mut ring_public_keys: Vec<String> = users_clone
                    .iter()
                    .filter(|u| {
                        u.id == author_id
                            || u.role == "platform_admin"
                            || (u.role == "admin" && u.workspace_id.as_ref() == Some(&workspace_id))
                            || u.workspace_id.as_ref() == Some(&workspace_id)
                    })
                    .filter_map(|u| u.public_key.clone())
                    .collect();
                ring_public_keys.sort();
                ring_public_keys.dedup();
                ring_public_keys.join(",")
            } else {
                author_user
                    .and_then(|u| u.public_key.clone())
                    .unwrap_or_default()
            };

            let ciphertext_bytes = const_hex::decode(&ciphertext_clone).unwrap_or_default();
            let data_hash = blake3::hash(&ciphertext_bytes);
            let data_hash_hex = const_hex::encode(data_hash.as_bytes());
            trust
                .verify_compliance_proof(
                    proof.to_string(),
                    author_id,
                    author_role,
                    data_hash_hex,
                    public_key_hex,
                )
                .unwrap_or(false)
        } else {
            false
        }
    });

    let author = users
        .iter()
        .find(|u| Some(u.id.clone()) == note.author_id)
        .and_then(|u| u.full_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    let is_author = note.author_id == Some(active_user.id.clone());
    let active_user_role = active_user.role.clone();
    let is_manager = active_user_role == "platform_admin" || active_user_role == "admin";
    let can_delete = is_author || is_manager;
    let note_id = note.id.clone();

    let is_history_expanded = Some(note.id.clone()) == *expanded_note_history_id.read();

    // Parse edit history
    let history_val: serde_json::Value =
        serde_json::from_str(&note.edit_history).unwrap_or_else(|_| serde_json::json!([]));
    let history_arr = history_val.as_array();
    let history_len = history_arr.map(|a| a.len()).unwrap_or(0);

    let date_str = if note.created_at.len() >= 10 {
        note.created_at[..10].to_string()
    } else {
        note.created_at.clone()
    };
    let time_str = if note.created_at.len() >= 19 {
        note.created_at[11..16].to_string()
    } else {
        String::new()
    };

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
                        "{t(\"notes-read-title\", &locale)}"
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
                                "{t(\"notes-read-written-by\", &locale)} {author}"
                            }
                            div { class: "text-xs text-muted-foreground/60",
                                "{t(\"notes-read-published\", &locale)} {date_str} {t(\"notes-read-at_time\", &locale)} {time_str}"
                            }
                        }
                    }
                }

                if is_manager && history_len > 0 {
                    button {
                        class: "bg-transparent border border-border rounded-lg text-xs font-bold cursor-pointer flex items-center text-amber-500 px-3.5 py-1.5 gap-1.5 transition-colors duration-150 mb-6",
                        onclick: move |_| expanded_note_history_id.set(if is_history_expanded { None } else { Some(note_id.clone()) }),
                        span { "⏳" }
                        span { "{t(\"notes-read-history-button\", &locale)} ({history_len})" }
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
                                span { "{t(\"notes-read-audit-log-title\", &locale)}" }
                            }
                            span { class: "font-medium text-[10px] opacity-70",
                                "{history_len} {t(\"notes-read-changes-recorded\", &locale)}"
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
                                                    span { class: "font-semibold", "{t(\"notes-read-edited-by\", &locale)}: {edited_by}" }
                                                    span { "{t(\"common-date\", &locale)}: {edited_at}" }
                                                }
                                                if let (Some(os), Some(ns)) = (old_subject, new_subject) {
                                                    div {
                                                        class: "flex items-center gap-1.5 text-xs",
                                                        span { class: "text-muted-foreground/60", "{t(\"notes-compose-subject-label\", &locale)}:" }
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
                                                                span { class: "text-[11px] text-muted-foreground/60 font-semibold", "{t(\"notes-read-diff-label\", &locale)}:" }
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
                h3 { class: "font-extrabold text-foreground mt-0 mb-5 text-2xl tracking-tight flex items-center gap-3 flex-wrap",
                    "{note.subject}"
                    if has_enc {
                        if *proof_verified.read() {
                            span {
                                class: "text-[10px] font-bold text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2 py-0.5 rounded flex items-center gap-1",
                                "✓ Zero-Copy ZKP Compliance Verified"
                            }
                        } else {
                            span {
                                class: "text-[10px] font-bold text-rose-400 bg-rose-500/10 border border-rose-500/20 px-2 py-0.5 rounded flex items-center gap-1",
                                "⚠ Invalid Compliance Proof"
                            }
                        }
                    }
                }
                if has_enc {
                    if let Some(plain_text) = decrypted_content.read().clone() {
                        div {
                            class: "text-sm text-foreground m-0 leading-relaxed whitespace-pre-wrap border-l-2 border-amber-500 pl-4 py-1",
                            "{plain_text}"
                        }
                    } else {
                        div {
                            class: "flex flex-col gap-4 p-6 border border-amber-500/30 bg-amber-500/5 rounded-xl text-sm box-border mb-6",
                            div { class: "flex items-center gap-3",
                                span { class: "text-2xl", "🔒" }
                                div { class: "flex flex-col gap-0.5",
                                    span { class: "font-bold text-foreground text-xs", "End-to-End Encrypted Content" }
                                    span { class: "text-[10px] text-muted-foreground", "This note is secured with hardware-backed Passkey envelope encryption." }
                                }
                            }
                            div { class: "flex flex-col gap-2",
                                label { class: "text-[9px] font-bold uppercase tracking-wider text-muted-foreground", "Enter Passkey Seed to Decrypt" }
                                div { class: "flex gap-2.5 items-center",
                                    crate::components::Input {
                                        class: "text-xs h-9 bg-background border border-border rounded px-3 flex-1 text-foreground",
                                        placeholder: "Passkey seed...",
                                        value: "{passkey_seed_input}",
                                        oninput: move |e: FormEvent| passkey_seed_input.set(e.value()),
                                    }
                                    button {
                                        class: "yntra-btn rounded-lg font-semibold px-4 py-2 cursor-pointer bg-amber-500 text-neutral-900 border-0 text-xs h-9 transition-colors hover:bg-amber-400",
                                        onclick: move |_| {
                                            let trust = yntra_core::ZkCryptoTrust::new();
                                            match trust.decrypt_workspace_field(passkey_seed_input.read().clone(), ciphertext.to_string()) {
                                                Ok(decrypted) => {
                                                    decrypted_content.set(Some(decrypted));
                                                    decryption_error.set(None);
                                                }
                                                Err(_) => {
                                                    decryption_error.set(Some("Decryption failed. Please verify your Passkey Seed.".to_string()));
                                                }
                                            }
                                        },
                                        "Decrypt"
                                    }
                                }
                                if let Some(err) = decryption_error.read().clone() {
                                    span { class: "text-xs font-semibold text-rose-500 animate-in fade-in duration-150", "{err}" }
                                }
                            }
                        }
                    }
                } else {
                    div {
                        class: "text-sm text-muted-foreground m-0 leading-relaxed whitespace-pre-wrap",
                        "{note.content}"
                    }
                }
            }
        }
    }
}
