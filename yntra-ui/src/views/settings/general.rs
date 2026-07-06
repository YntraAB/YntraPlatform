use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{update_workspace_general, Workspace};

// A standard base64 encoder to parse image bytes to data URLs for local-first persistence
fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = match chunk.len() {
            3 => ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32),
            2 => ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8),
            1 => (chunk[0] as u32) << 16,
            _ => unreachable!(),
        };
        let c1 = CHARSET[((b >> 18) & 63) as usize] as char;
        let c2 = CHARSET[((b >> 12) & 63) as usize] as char;
        let c3 = if chunk.len() > 1 { CHARSET[((b >> 6) & 63) as usize] as char } else { '=' };
        let c4 = if chunk.len() > 2 { CHARSET[(b & 63) as usize] as char } else { '=' };
        result.push(c1);
        result.push(c2);
        result.push(c3);
        result.push(c4);
    }
    result
}

#[derive(Props, Clone)]
pub struct GeneralSettingsProps {
    pub settings_name: Signal<String>,
    pub settings_brand_color: Signal<String>,
    pub settings_logo_url: Signal<String>,
    pub settings_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub workspace: Workspace,
    pub locale: String,
}

impl PartialEq for GeneralSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn GeneralSettings(props: GeneralSettingsProps) -> Element {
    let mut settings_name = props.settings_name;
    let mut settings_brand_color = props.settings_brand_color;
    let mut settings_logo_url = props.settings_logo_url;
    let mut settings_save_status = props.settings_save_status;
    let mut db_trigger = props.db_trigger;

    let mut selected_image = use_signal(|| Option::<String>::None);
    let mut cropper_open = use_signal(|| false);
    let mut is_uploading = use_signal(|| false);

    // Read settings from workspace settings JSON
    let settings_val: serde_json::Value = serde_json::from_str(&props.workspace.settings).unwrap_or_default();
    let modules_val: serde_json::Value = serde_json::from_str(&props.workspace.modules_active).unwrap_or_default();
    let is_school = modules_val.get("school").and_then(|v| v.as_bool()).unwrap_or(false);
    
    let mut language = use_signal(|| {
        let raw = settings_val.get("language").and_then(|v| v.as_str()).unwrap_or("US");
        match raw.to_lowercase().as_str() {
            "sv" | "se" => "SE",
            "no" | "nb" | "nn" => "NO",
            "da" | "dk" => "DK",
            "fi" => "FI",
            _ => "US",
        }.to_string()
    });
    let mut timezone = use_signal(|| {
        settings_val.get("timezone").and_then(|v| v.as_str()).unwrap_or("Europe/Stockholm").to_string()
    });
    let mut week_start = use_signal(|| {
        settings_val.get("week_start").and_then(|v| v.as_i64()).unwrap_or(1) as i32
    });
    let mut template = use_signal(|| {
        settings_val.get("template").and_then(|v| v.as_str()).unwrap_or("care").to_string()
    });
    let mut grading_system = use_signal(|| {
        settings_val.get("grading_system").and_then(|v| v.as_str()).unwrap_or("A-F").to_string()
    });
    let mut late_policy = use_signal(|| {
        settings_val.get("late_policy").and_then(|v| v.as_str()).unwrap_or("none").to_string()
    });
    let mut target_region = use_signal(|| {
        settings_val.get("target_region").and_then(|v| v.as_str()).unwrap_or("EU").to_string()
    });

    let state = use_context::<crate::state::AppState>();
    // Helper functions that clone required values to avoid borrow checker errors
    let workspace_id = props.workspace.id.clone();
    let save_identity = {
        let workspace_id = workspace_id.clone();
        let user_id = state.active_user_id.read().clone();
        move |name: String, color: String, logo: Option<String>| {
            let name_trimmed = name.trim().to_string();
            if !name_trimmed.is_empty() {
                settings_save_status.set("saving".to_string());
                let ws_id = workspace_id.clone();
                let requester_uid = user_id.clone();
                spawn(async move {
                    let _ = update_workspace_general(requester_uid, ws_id, name_trimmed, color, logo).await;
                });
                let current_trig = *db_trigger.read();
                db_trigger.set(current_trig + 1);
                settings_save_status.set("saved".to_string());
            }
        }
    };

    let save_settings = {
        let workspace_settings_raw = props.workspace.settings.clone();
        let workspace_id = workspace_id.clone();
        let user_id = state.active_user_id.read().clone();
        move || {
            settings_save_status.set("saving".to_string());
            let mut settings_map: serde_json::Value = serde_json::from_str(&workspace_settings_raw).unwrap_or_default();
            settings_map["language"] = serde_json::json!((*language.read()).clone());
            settings_map["timezone"] = serde_json::json!((*timezone.read()).clone());
            settings_map["week_start"] = serde_json::json!(*week_start.read());
            settings_map["template"] = serde_json::json!((*template.read()).clone());
            settings_map["grading_system"] = serde_json::json!((*grading_system.read()).clone());
            settings_map["late_policy"] = serde_json::json!((*late_policy.read()).clone());
            settings_map["target_region"] = serde_json::json!((*target_region.read()).clone());
            
            let settings_str = serde_json::to_string(&settings_map).unwrap_or_default();
            let ws_id = workspace_id.clone();
            let requester_uid = user_id.clone();
            spawn(async move {
                let _ = yntra_core::update_workspace_settings(requester_uid, ws_id, settings_str).await;
            });
            
            let current_trig = *db_trigger.read();
            db_trigger.set(current_trig + 1);
            settings_save_status.set("saved".to_string());
        }
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "grid grid-cols-1 items-stretch gap-6 md:grid-cols-2",
                
                // 1. Organization Identity Card
                components::Card { class: "flex h-full flex-col overflow-hidden border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-md",
                    components::CardHeader { class: "pb-4",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary shadow-inner",
                                components::LucideIcon { name: "building-2", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold tracking-tight",
                                    "{t(\"settings-organization-identity\", &props.locale)}"
                                }
                                components::CardDescription { class: "text-xs",
                                    "{t(\"settings-identity-desc\", &props.locale)}"
                                }
                            }
                        }
                    }
                    components::CardContent { class: "flex-1 space-y-6",
                        div { class: "grid grid-cols-1 gap-6 sm:grid-cols-2",
                            div { class: "space-y-6",
                                div { class: "space-y-2",
                                    label {
                                        r#for: "org-name",
                                        class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                        "{t(\"settings-organization-name\", &props.locale)}"
                                    }
                                    div { class: "group relative",
                                        input {
                                            id: "org-name",
                                            class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40 transition-all duration-300 focus:ring-2 focus:ring-primary/20",
                                            value: "{settings_name}",
                                            oninput: move |e| settings_name.set(e.value()),
                                            onblur: {
                                                let save_identity = save_identity.clone();
                                                move |_| {
                                                    let mut save_identity = save_identity.clone();
                                                    save_identity((*settings_name.read()).clone(), (*settings_brand_color.read()).clone(), if settings_logo_url.read().is_empty() { None } else { Some((*settings_logo_url.read()).clone()) });
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "space-y-2",
                                    label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                        "{t(\"settings-workspace-logo\", &props.locale)}"
                                    }
                                    div { class: "flex flex-col gap-3",
                                        div { class: "flex gap-2",
                                            label {
                                                r#for: "logo-upload-input",
                                                class: "flex h-11 flex-1 items-center justify-center gap-2 pl-2 rounded-xl bg-primary text-xs font-bold text-primary-foreground shadow-lg shadow-primary/20 transition-all hover:scale-[1.02] active:scale-95 disabled:opacity-50 cursor-pointer",
                                                if *is_uploading.read() {
                                                    components::LucideIcon { name: "loader-2", class: "h-4 w-4 animate-spin" }
                                                    span { "{t(\"common-loading\", &props.locale)}" }
                                                } else {
                                                    components::LucideIcon { name: "upload", class: "h-4 w-4" }
                                                    span { "{t(\"settings-upload-new-logo\", &props.locale)}" }
                                                }
                                            }
                                            input {
                                                r#type: "file",
                                                id: "logo-upload-input",
                                                accept: "image/*",
                                                class: "hidden",
                                                disabled: *is_uploading.read(),
                                                onchange: move |evt| {
                                                    spawn(async move {
                                                        let files = evt.files();
                                                        if !files.is_empty() {
                                                            is_uploading.set(true);
                                                            if let Ok(bytes) = files[0].read_bytes().await {
                                                                let base64_str = base64_encode(bytes.as_ref());
                                                                let data_url = format!("data:image/png;base64,{}", base64_str);
                                                                selected_image.set(Some(data_url));
                                                                cropper_open.set(true);
                                                            }
                                                            is_uploading.set(false);
                                                        }
                                                    });
                                                }
                                            }
                                            if !settings_logo_url.read().is_empty() {
                                                button {
                                                    class: "flex h-11 w-11 items-center justify-center rounded-xl border border-red-500/20 bg-red-500/10 text-red-500 transition-all hover:bg-red-500/20 active:scale-95",
                                                    onclick: {
                                                        let save_identity = save_identity.clone();
                                                        move |_| {
                                                            let mut save_identity = save_identity.clone();
                                                            settings_logo_url.set(String::new());
                                                            save_identity((*settings_name.read()).clone(), (*settings_brand_color.read()).clone(), None);
                                                        }
                                                    },
                                                    components::LucideIcon { name: "trash-2", class: "h-4 w-4" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Logo Preview
                            div { class: "flex items-center justify-center rounded-2xl border border-dashed border-border/40 bg-background/20 p-4",
                                if !settings_logo_url.read().is_empty() {
                                    img {
                                        src: "{settings_logo_url}",
                                        class: "max-h-28 max-w-full rounded-lg object-contain shadow-md",
                                    }
                                } else {
                                    div { class: "text-center text-muted-foreground/60",
                                        components::LucideIcon { name: "image", class: "mx-auto mb-2 h-8 w-8 opacity-40" }
                                        p { class: "text-xs", "{t(\"settings-no-logo\", &props.locale)}" }
                                    }
                                }
                            }
                        }

                        div { class: "border-t border-border/40 pt-6",
                            div { class: "mb-4 flex items-center justify-between",
                                div {
                                    h4 { class: "text-[11px] font-bold uppercase tracking-[0.2em] text-foreground",
                                        "{t(\"settings-workspace-brand-color\", &props.locale)}"
                                    }
                                    p { class: "text-[10px] text-muted-foreground",
                                        "{t(\"settings-workspace-brand-color-desc\", &props.locale)}"
                                    }
                                }
                                components::HexColorPicker {
                                    value: settings_brand_color.read().clone(),
                                    onchange: {
                                        let save_identity = save_identity.clone();
                                        move |color: String| {
                                            let mut save_identity = save_identity.clone();
                                            settings_brand_color.set(color.clone());
                                            save_identity((*settings_name.read()).clone(), color, if settings_logo_url.read().is_empty() { None } else { Some((*settings_logo_url.read()).clone()) });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Preferences & Regional Settings
                components::Card { class: "flex h-full flex-col overflow-hidden border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-md",
                    components::CardHeader { class: "pb-4",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary shadow-inner",
                                components::LucideIcon { name: "sliders", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold tracking-tight",
                                    "{t(\"settings-regional-preferences\", &props.locale)}"
                                }
                                components::CardDescription { class: "text-xs",
                                    "{t(\"settings-preferences-desc\", &props.locale)}"
                                }
                            }
                        }
                    }
                    components::CardContent { class: "grid flex-1 grid-cols-1 gap-4 sm:grid-cols-2",
                        // Language Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "{t(\"settings-language\", &props.locale)}"
                            }
                            select {
                                class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{language}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |e| {
                                        let mut save_settings = save_settings.clone();
                                        let val = e.value();
                                        language.set(val.clone());
                                        save_settings();
                                    }
                                },
                                option { value: "SE", "{t(\"settings-languages-sv\", &props.locale)}" }
                                option { value: "NO", "{t(\"settings-languages-no\", &props.locale)}" }
                                option { value: "DK", "{t(\"settings-languages-da\", &props.locale)}" }
                                option { value: "FI", "{t(\"settings-languages-fi\", &props.locale)}" }
                                option { value: "US", "{t(\"settings-languages-en\", &props.locale)}" }
                            }
                        }

                        // Target Region Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "Target Region (Compliance)"
                            }
                            select {
                                class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{target_region}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |e| {
                                        let mut save_settings = save_settings.clone();
                                        let val = e.value();
                                        target_region.set(val.clone());
                                        save_settings();
                                    }
                                },
                                option { value: "EU", "EU (Default)" }
                                option { value: "SE", "Sweden (SE)" }
                                option { value: "NO", "Norway (NO)" }
                                option { value: "DK", "Denmark (DK)" }
                                option { value: "FI", "Finland (FI)" }
                                option { value: "US-FED", "US Federal (FLSA)" }
                                option { value: "US-CA", "US California" }
                                option { value: "US-CO", "US Colorado" }
                                option { value: "US-NV", "US Nevada" }
                                option { value: "US-AK", "US Alaska" }
                            }
                        }

                        // Timezone Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "{t(\"settings-timezone\", &props.locale)}"
                            }
                            select {
                                class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{timezone}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |e| {
                                        let mut save_settings = save_settings.clone();
                                        let val = e.value();
                                        timezone.set(val.clone());
                                        save_settings();
                                    }
                                },
                                option { value: "Europe/Stockholm", "{t(\"settings-timezones-stockholm\", &props.locale)}" }
                                option { value: "UTC", "{t(\"settings-timezones-utc\", &props.locale)}" }
                            }
                        }

                        // Week Start Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "{t(\"settings-week-start\", &props.locale)}"
                            }
                            select {
                                class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{week_start}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |e| {
                                        let mut save_settings = save_settings.clone();
                                        if let Ok(val) = e.value().parse::<i32>() {
                                            week_start.set(val);
                                            save_settings();
                                        }
                                    }
                                },
                                option { value: "1", "{t(\"settings-monday\", &props.locale)}" }
                                option { value: "0", "{t(\"settings-sunday\", &props.locale)}" }
                            }
                        }

                        // Workspace Template Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "Workspace Template"
                            }
                            select {
                                class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{template}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |e| {
                                        let mut save_settings = save_settings.clone();
                                        let val = e.value();
                                        template.set(val.clone());
                                        save_settings();
                                    }
                                },
                                option { value: "care", "Care & Assistance" }
                                option { value: "moving", "Moving Company" }
                                option { value: "general", "General Operations" }
                            }
                        }

                        // Grading System (only shown if school module is active)
                        if is_school {
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Grading System / Scale"
                                }
                                select {
                                    class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40",
                                    value: "{grading_system}",
                                    onchange: {
                                        let save_settings = save_settings.clone();
                                        move |e| {
                                            let mut save_settings = save_settings.clone();
                                            let val = e.value();
                                            grading_system.set(val.clone());
                                            save_settings();
                                        }
                                    },
                                    option { value: "A-F", "A-F (Letter Grades)" }
                                    option { value: "1-10", "1-10 (Numeric Scale)" }
                                    option { value: "1-100", "0-100 (Percentage Scale)" }
                                    option { value: "U-G-VG", "U, G, VG (Swedish University Scale)" }
                                    option { value: "U-G", "U, G (Swedish Pass/Fail)" }
                                    option { value: "U-3-4-5", "U, 3, 4, 5 (Swedish Engineering)" }
                                }
                            }
                        }

                        // Late Submission Policy (only shown if school module is active)
                        if is_school {
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Default Late Submission Policy"
                                }
                                select {
                                    class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40",
                                    value: "{late_policy}",
                                    onchange: {
                                        let save_settings = save_settings.clone();
                                        move |e| {
                                            let mut save_settings = save_settings.clone();
                                            let val = e.value();
                                            late_policy.set(val.clone());
                                            save_settings();
                                        }
                                    },
                                    option { value: "none", "None (No Penalties)" }
                                    option { value: "hard_deadline", "Hard Deadline (Block Late Submissions)" }
                                    option { value: "penalty_5", "5% Daily Deduction Penalty" }
                                    option { value: "penalty_10", "10% Daily Deduction Penalty" }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Image Cropper Overlay
        if *cropper_open.read() {
            if let Some(img_data) = selected_image.read().clone() {
                components::ImageCropperDialog {
                    open: *cropper_open.read(),
                    image_data: img_data,
                    onclose: move |_| {
                        cropper_open.set(false);
                        selected_image.set(None);
                    },
                    oncrop: {
                        let save_identity = save_identity.clone();
                        move |cropped_url: String| {
                            let mut save_identity = save_identity.clone();
                            settings_logo_url.set(cropped_url.clone());
                            save_identity((*settings_name.read()).clone(), (*settings_brand_color.read()).clone(), Some(cropped_url));
                        }
                    }
                }
            }
        }
    }
}
