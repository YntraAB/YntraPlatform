use super::super::utils::{
    AdvancedAttachment, base64_encode, compute_mock_hash, encrypt_field_with_proof,
    format_file_size, is_deadline_passed, parse_submission_content_and_advanced_attachment,
};
use crate::components::{Button, LucideIcon};
use crate::locales::{t, t_with_args};
use dioxus::html::HasFileData;
use dioxus::prelude::*;
use yntra_core::{Assignment, Submission, save_submission};

#[component]
pub fn StudentClassworkTab(
    mut homework_filter: Signal<String>,
    filtered_assignments: Vec<Assignment>,
    student_submissions: Vec<Submission>,
    mut submitting_map: Signal<std::collections::HashSet<String>>,
    mut drag_active: Signal<std::collections::HashMap<String, bool>>,
    mut submission_files: Signal<std::collections::HashMap<String, AdvancedAttachment>>,
    mut assignment_inputs: Signal<std::collections::HashMap<String, String>>,
    locale: String,
    active_user_id: String,
    workspace_id: String,
    student_profile_id: String,
    mut db_trigger: Signal<u32>,
) -> Element {
    let state = use_context::<crate::state::AppState>();
    let toast = dioxus_primitives::toast::use_toast();
    let homework_filter_val = homework_filter.read().clone();

    rsx! {
        div { class: "space-y-4 animate-in fade-in duration-300",
            // Homework filter tabs
            div { class: "flex bg-muted/60 p-0.5 rounded-xl border border-border/40 text-[10px] font-bold w-max",
                button {
                    class: format!(
                        "px-3 py-1.5 rounded-lg transition-all {}",
                        if homework_filter_val == "todo" { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| homework_filter.set("todo".to_string()),
                    {t("school-student-todo-tab", &locale)}
                }
                button {
                    class: format!(
                        "px-3 py-1.5 rounded-lg transition-all {}",
                        if homework_filter_val == "done" { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| homework_filter.set("done".to_string()),
                    {t("school-student-done-tab", &locale)}
                }
                button {
                    class: format!(
                        "px-3 py-1.5 rounded-lg transition-all {}",
                        if homework_filter_val == "materials" { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| homework_filter.set("materials".to_string()),
                    {t("school-student-materials-tab", &locale)}
                }
            }

            if filtered_assignments.is_empty() {
                div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl italic bg-muted/5",
                    {t("school-student-no-homework", &locale)}
                }
            } else {
                div { class: "space-y-4",
                    for a in filtered_assignments.into_iter() {
                        {
                            let a_id = a.id.clone();
                            let existing_sub = student_submissions.iter().find(|sub| sub.assignment_id == a_id).cloned();
                            let (desc_text, attachment) = parse_submission_content_and_advanced_attachment(&a.description);
                            rsx! {
                                div { key: "{a.id}", class: "p-5 border border-border rounded-xl bg-background space-y-4 shadow-sm",
                                    div { class: "flex items-start justify-between gap-4 flex-wrap sm:flex-nowrap",
                                        div { class: "space-y-1.5",
                                            div { class: "flex items-center gap-2 flex-wrap",
                                                span { class: "font-bold text-foreground text-sm", "{a.title}" }
                                                span { class: "text-[10px] text-muted-foreground font-semibold flex items-center gap-1",
                                                    LucideIcon { name: "calendar", class: "h-3.5 w-3.5" }
                                                    if a.due_date.contains("No due date") || a.due_date.is_empty() {
                                                        span { class: "text-muted-foreground/60 font-medium", "No due date" }
                                                    } else if a.due_date.contains("(Strict)") {
                                                        if is_deadline_passed(&a.due_date) {
                                                            span { class: "text-red-500 font-semibold", {t_with_args("school-student-strict-passed", &locale, &[("date", &a.due_date)])} }
                                                        } else {
                                                            span { class: "text-red-500/80 font-medium", {t_with_args("school-student-strict-due", &locale, &[("date", &a.due_date)])} }
                                                        }
                                                    } else if a.due_date.contains("(Flexible)") {
                                                        if is_deadline_passed(&a.due_date) {
                                                            span { class: "text-amber-500 font-semibold", {t_with_args("school-student-flexible-late", &locale, &[("date", &a.due_date)])} }
                                                        } else {
                                                            span { class: "text-amber-500/80 font-medium", {t_with_args("school-student-flexible-due", &locale, &[("date", &a.due_date)])} }
                                                        }
                                                    } else {
                                                        span { class: "text-muted-foreground font-medium", "Due: {a.due_date}" }
                                                    }
                                                }
                                            }
                                            if !desc_text.is_empty() {
                                                div { class: "text-muted-foreground break-words font-medium leading-relaxed max-w-2xl text-xs", "{desc_text}" }
                                            }
                                            if let Some(staged) = attachment {
                                                a {
                                                    href: "{staged.dataurl}",
                                                    download: "{staged.filename}",
                                                    class: "flex items-center gap-2 p-1.5 bg-card/45 border border-primary/20 hover:bg-card/75 rounded-lg no-underline text-foreground cursor-pointer transition-all mt-2 w-fit max-w-sm",
                                                    LucideIcon { name: "file-text", class: "h-3.5 w-3.5 text-primary shrink-0" }
                                                    span { class: "text-[10px] font-semibold truncate max-w-[150px]", "{staged.filename}" }
                                                    span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold", "{staged.size_str}" }
                                                    LucideIcon { name: "download", class: "h-3 w-3 text-muted-foreground ml-1.5 shrink-0" }
                                                }
                                            }
                                        }
                                        if a.max_points == 0 {
                                            span { class: "text-[10px] font-bold bg-primary/10 text-primary border border-primary/20 px-2.5 py-0.5 rounded-full uppercase tracking-wider",
                                                "Material"
                                            }
                                        } else {
                                            span { class: "text-xs font-bold bg-primary/10 text-primary px-2.5 py-1 rounded-full",
                                                "{a.max_points} pts"
                                            }
                                        }
                                    }
                                    if let Some(ref sub) = existing_sub {
                                        {
                                            let (ans_text, attachment) = parse_submission_content_and_advanced_attachment(&sub.content);
                                            rsx! {
                                                div { class: "p-3.5 rounded-lg bg-muted/40 border border-border/50 text-xs space-y-2",
                                                    div { class: "font-semibold text-muted-foreground", {t("school-submitted-answers", &locale)} }
                                                    div { class: "text-foreground font-medium", "{ans_text}" }
                                                    if let Some(staged) = attachment {
                                                        a {
                                                            href: "{staged.dataurl}",
                                                            download: "{staged.filename}",
                                                            class: "flex flex-col gap-2 p-3 bg-card/45 border border-primary/20 rounded-xl max-w-sm no-underline text-foreground cursor-pointer hover:bg-card/75 transition-colors mt-2",
                                                            div { class: "flex items-center justify-between",
                                                                div { class: "flex items-center gap-1.5 min-w-0",
                                                                    LucideIcon { name: "file-check", class: "h-4 w-4 text-primary shrink-0" }
                                                                    span { class: "text-[10px] text-foreground font-semibold truncate", "{staged.filename}" }
                                                                }
                                                                LucideIcon { name: "download", class: "h-3.5 w-3.5 text-muted-foreground" }
                                                            }
                                                            div { class: "flex flex-wrap gap-1.5 items-center mt-1",
                                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold uppercase", "{staged.size_str}" }
                                                                if staged.e2ee {
                                                                    span { class: "text-[8px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-500 font-bold flex items-center gap-0.5",
                                                                        LucideIcon { name: "shield-check", class: "h-2.5 w-2.5" }
                                                                        "E2EE SECURED"
                                                                    }
                                                                }
                                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-medium truncate max-w-[120px]", "sha256:{staged.sha256}" }
                                                            }
                                                        }
                                                    }
                                                    div { class: "flex items-center gap-2 border-t border-border/30 pt-2 mt-2",
                                                        span { class: "text-[10px] uppercase font-bold text-muted-foreground", "Status: " }
                                                        if let Some(ref g) = sub.grade {
                                                            span { class: "text-[10px] px-2 py-0.5 rounded-full font-bold bg-green-500/10 text-green-600 border border-green-500/20 uppercase",
                                                                "Graded: {g}"
                                                            }
                                                        } else {
                                                            span { class: "text-[10px] px-2 py-0.5 rounded-full font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20 uppercase",
                                                                {t("school-waiting-grade", &locale)}
                                                            }
                                                        }
                                                    }
                                                    if let Some(ref feedback) = sub.feedback {
                                                        if !feedback.is_empty() {
                                                            div { class: "text-[11px] text-muted-foreground italic pl-2 border-l border-primary/30 mt-1.5",
                                                                span { class: "font-bold not-italic", {t("school-teacher-feedback", &locale)} }
                                                                " {feedback}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if a.max_points == 0 {
                                        div { class: "p-3.5 rounded-xl border border-primary/20 bg-primary/10 text-xs text-primary flex items-center gap-2 font-medium",
                                            LucideIcon { name: "info", size: "14" }
                                            span { "This is a study material provided for reference. No submission is required." }
                                        }
                                    } else if is_deadline_passed(&a.due_date) {
                                        div { class: "p-3.5 rounded-xl border border-red-200/50 bg-red-500/5 text-xs text-red-600 flex items-center gap-2 font-medium",
                                            LucideIcon { name: "lock", size: "14" }
                                            span { {t("school-student-closed-message", &locale)} }
                                        }
                                    } else {
                                        // Answer submission input box with Drag-and-Drop uploader
                                        {
                                            let is_drag_over = *drag_active.read().get(&a_id).unwrap_or(&false);
                                            let staged_file = submission_files.read().get(&a_id).cloned();
                                            let drag_border_class = if is_drag_over { "border-primary bg-primary/5 shadow-md scale-[1.01]" } else { "border-border/60 bg-muted/20" };

                                            rsx! {
                                                div {
                                                    class: "flex flex-col gap-3 p-4 rounded-xl border border-dashed transition-all duration-200 {drag_border_class}",
                                                    ondragover: {
                                                        let a_id_c = a_id.clone();
                                                        move |evt: DragEvent| {
                                                            evt.prevent_default();
                                                            drag_active.write().insert(a_id_c.clone(), true);
                                                        }
                                                    },
                                                    ondragleave: {
                                                        let a_id_c = a_id.clone();
                                                        move |evt: DragEvent| {
                                                            evt.prevent_default();
                                                            drag_active.write().insert(a_id_c.clone(), false);
                                                        }
                                                    },
                                                    ondrop: {
                                                        let a_id_c = a_id.clone();
                                                        let a_id_f = a_id.clone();
                                                        move |evt: DragEvent| {
                                                            evt.prevent_default();
                                                            drag_active.write().insert(a_id_c.clone(), false);
                                                            let files = evt.files();
                                                            let a_id_val = a_id_f.clone();
                                                            spawn(async move {
                                                                if !files.is_empty() {
                                                                    let file_name = files[0].name();
                                                                    if let Ok(bytes) = files[0].read_bytes().await {
                                                                        let size_str = format_file_size(bytes.len());
                                                                        let sha256 = compute_mock_hash(&bytes);
                                                                        let base64_str = base64_encode(bytes.as_ref());
                                                                        let data_url = format!("data:application/octet-stream;base64,{}", base64_str);
                                                                        submission_files.write().insert(a_id_val, AdvancedAttachment {
                                                                            filename: file_name,
                                                                            size_str,
                                                                            sha256,
                                                                            e2ee: true,
                                                                            dataurl: data_url,
                                                                        });
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    span { class: "text-[11px] font-bold text-muted-foreground", {t("school-student-write-answer", &locale)} }
                                                    textarea {
                                                        class: "w-full min-h-[70px] rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                                        placeholder: t("school-student-type-placeholder", &locale),
                                                        value: assignment_inputs.read().get(&a_id).cloned().unwrap_or_default(),
                                                        oninput: {
                                                            let a_id_c = a_id.clone();
                                                            move |evt: FormEvent| {
                                                                assignment_inputs.write().insert(a_id_c.clone(), evt.value());
                                                            }
                                                        },
                                                    }

                                                    // Drag and drop / file selector connector
                                                    div { class: "border border-dashed border-border/85 rounded-lg p-4 flex flex-col items-center justify-center bg-background/50 hover:bg-background/85 transition-colors cursor-pointer relative",
                                                        input {
                                                            r#type: "file",
                                                            class: "absolute inset-0 opacity-0 cursor-pointer z-10",
                                                            onchange: {
                                                                let a_id_c = a_id.clone();
                                                                move |evt| {
                                                                    let a_id_f = a_id_c.clone();
                                                                    let files = evt.files();
                                                                    spawn(async move {
                                                                        if !files.is_empty() {
                                                                            let file_name = files[0].name();
                                                                            if let Ok(bytes) = files[0].read_bytes().await {
                                                                                let size_str = format_file_size(bytes.len());
                                                                                let sha256 = compute_mock_hash(&bytes);
                                                                                let base64_str = base64_encode(bytes.as_ref());
                                                                                let data_url = format!("data:application/octet-stream;base64,{}", base64_str);
                                                                                submission_files.write().insert(a_id_f.clone(), AdvancedAttachment {
                                                                                    filename: file_name,
                                                                                    size_str,
                                                                                    sha256,
                                                                                    e2ee: true,
                                                                                    dataurl: data_url,
                                                                                });
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            }
                                                        }
                                                        LucideIcon { name: "upload-cloud", class: "h-5 w-5 text-primary mb-1.5" }
                                                        span { class: "text-[10px] text-foreground font-semibold", {t("school-student-drag-drop", &locale)} }
                                                        span { class: "text-[9px] text-muted-foreground mt-0.5", {t("school-student-upload-formats", &locale)} }
                                                    }

                                                    if let Some(staged) = staged_file {
                                                        div { class: "flex flex-col gap-2 p-3 bg-card/45 border border-primary/20 rounded-xl",
                                                            div { class: "flex items-center justify-between",
                                                                div { class: "flex items-center gap-1.5 min-w-0",
                                                                    LucideIcon { name: "file-check", class: "h-4 w-4 text-primary shrink-0" }
                                                                    span { class: "text-[10px] text-foreground font-semibold truncate", "{staged.filename}" }
                                                                }
                                                                button {
                                                                    class: "bg-transparent border-0 text-muted-foreground hover:text-red-500 cursor-pointer p-0.5 rounded transition-colors",
                                                                    onclick: {
                                                                        let a_id_c = a_id.clone();
                                                                        move |_| {
                                                                            submission_files.write().remove(&a_id_c);
                                                                        }
                                                                    },
                                                                    LucideIcon { name: "x", class: "h-3.5 w-3.5" }
                                                                }
                                                            }
                                                            div { class: "flex flex-wrap gap-1.5 items-center mt-1",
                                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold uppercase", "{staged.size_str}" }
                                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-500 font-bold flex items-center gap-0.5",
                                                                    LucideIcon { name: "shield-check", class: "h-2.5 w-2.5" }
                                                                    "E2EE SECURED"
                                                                }
                                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-medium truncate max-w-[120px]", "sha256:{staged.sha256}" }
                                                            }
                                                        }
                                                    }

                                                    Button {
                                                        class: "text-xs px-3 h-8 self-end font-semibold flex items-center gap-1.5 mt-2",
                                                        disabled: submitting_map.read().contains(&a_id),
                                                        onclick: {
                                                            let a_id = a_id.clone();
                                                            let s_id = student_profile_id.clone();
                                                            let uid_c = active_user_id.clone();
                                                            let ws_c = workspace_id.clone();
                                                            let db_trigger = db_trigger.clone();
                                                            let toast = toast.clone();
                                                            let locale_c = locale.clone();
                                                            move |_| {
                                                                let ans = assignment_inputs.read().get(&a_id).cloned().unwrap_or_default();
                                                                let staged = submission_files.read().get(&a_id).cloned();
                                                                let final_content = match staged.as_ref() {
                                                                    Some(staged_f) => format!(
                                                                        "{}\n[Attachment: {} | {} | {} | {} | blob://{}]",
                                                                        ans, staged_f.filename, staged_f.size_str, staged_f.sha256, staged_f.e2ee, staged_f.sha256
                                                                    ),
                                                                    None => ans.clone(),
                                                                };
                                                                if !ans.is_empty() || staged.is_some() {
                                                                    let role = state.active_user_role.read().clone();
                                                                    let u_id = state.active_user_id.read().clone();
                                                                    let proof = yntra_core::ZkCryptoTrust::new()
                                                                        .generate_role_proof(state.get_passkey_seed(), u_id.clone(), role.clone())
                                                                        .ok();
                                                                    let seed_val = state.get_passkey_seed();
                                                                    let enc_content = encrypt_field_with_proof(&seed_val, &final_content, &u_id, &role);
                                                                    let sub_rec = Submission {
                                                                        id: uuid::Uuid::new_v4().to_string(),
                                                                        workspace_id: ws_c.clone(),
                                                                        assignment_id: a_id.clone(),
                                                                        student_id: s_id.clone(),
                                                                        content: enc_content,
                                                                        grade: None,
                                                                        feedback: None,
                                                                        submitted_at: chrono::Utc::now().to_rfc3339(),
                                                                        updated_at: 0,
                                                                    };
                                                                    let uid_sub = uid_c.clone();
                                                                    let a_id_clear = a_id.clone();
                                                                    let mut db_trigger_c = db_trigger.clone();
                                                                    let toast_c = toast.clone();
                                                                    let locale_sub = locale_c.clone();
                                                                    let ws_blob = ws_c.clone();
                                                                    let staged_blob = staged.clone();
                                                                    submitting_map.write().insert(a_id.clone());
                                                                    spawn(async move {
                                                                        if let Some(staged_f) = staged_blob {
                                                                            let _ = yntra_core::save_blob(uid_sub.clone(), staged_f.sha256, ws_blob, staged_f.dataurl).await;
                                                                        }
                                                                        match save_submission(uid_sub, sub_rec, proof).await {
                                                                            Ok(_) => {
                                                                                toast_c.success(
                                                                                    t("school-submission-success-title", &locale_sub),
                                                                                    dioxus_primitives::toast::ToastOptions::new().description(t("school-submission-success-desc", &locale_sub))
                                                                                );
                                                                                assignment_inputs.write().insert(a_id_clear.clone(), String::new());
                                                                                submission_files.write().remove(&a_id_clear);
                                                                                let current = *db_trigger_c.read();
                                                                                db_trigger_c.set(current + 1);
                                                                            }
                                                                            Err(e) => {
                                                                                let user_err = crate::utils::map_error(&e);
                                                                                toast_c.error(
                                                                                    user_err.title,
                                                                                    dioxus_primitives::toast::ToastOptions::new().description(user_err.description)
                                                                                );
                                                                            }
                                                                        }
                                                                        submitting_map.write().remove(&a_id_clear);
                                                                    });
                                                                }
                                                            }
                                                        },
                                                        LucideIcon { name: "send", class: "h-3.5 w-3.5" }
                                                        {
                                                            if submitting_map.read().contains(&a_id) {
                                                                "Submitting...".to_string()
                                                            } else {
                                                                t("school-submit-answer", &locale)
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
}
