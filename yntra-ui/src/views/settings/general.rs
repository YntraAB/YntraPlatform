use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{Workspace, update_workspace_general};

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
        let c3 = if chunk.len() > 1 {
            CHARSET[((b >> 6) & 63) as usize] as char
        } else {
            '='
        };
        let c4 = if chunk.len() > 2 {
            CHARSET[(b & 63) as usize] as char
        } else {
            '='
        };
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
    let settings_val: serde_json::Value =
        serde_json::from_str(&props.workspace.settings).unwrap_or_default();
    let modules_val: serde_json::Value =
        serde_json::from_str(&props.workspace.modules_active).unwrap_or_default();
    let is_school = modules_val
        .get("school")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut language = use_signal(|| {
        let raw = settings_val
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("en");
        match raw.to_lowercase().as_str() {
            "sv" | "se" => "sv",
            "no" | "nb" | "nn" => "no",
            "da" | "dk" => "da",
            "fi" => "fi",
            _ => "en",
        }
        .to_string()
    });
    let mut timezone = use_signal(|| {
        settings_val
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("Europe/Stockholm")
            .to_string()
    });
    let mut week_start = use_signal(|| {
        settings_val
            .get("week_start")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32
    });
    let template = use_signal(|| {
        settings_val
            .get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("care")
            .to_string()
    });
    let mut grading_system = use_signal(|| {
        settings_val
            .get("grading_system")
            .and_then(|v| v.as_str())
            .unwrap_or("A-F")
            .to_string()
    });
    let mut late_policy = use_signal(|| {
        settings_val
            .get("late_policy")
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_string()
    });
    let mut target_region = use_signal(|| {
        settings_val
            .get("target_region")
            .and_then(|v| v.as_str())
            .unwrap_or("EU")
            .to_string()
    });

    let mut base_rate = use_signal(|| {
        settings_val
            .get("moving_base_rate_per_m3")
            .and_then(|v| v.as_f64())
            .unwrap_or(500.0)
    });
    let mut distance_fee = use_signal(|| {
        settings_val
            .get("moving_distance_fee_flat")
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0)
    });
    let mut stairs_surcharge = use_signal(|| {
        settings_val
            .get("moving_stairs_surcharge_per_floor")
            .and_then(|v| v.as_f64())
            .unwrap_or(300.0)
    });
    let mut packing_fee = use_signal(|| {
        settings_val
            .get("moving_packing_supplies_fee_per_m3")
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0)
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
                    let _ =
                        update_workspace_general(requester_uid, ws_id, name_trimmed, color, logo)
                            .await;
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
            let mut settings_map: serde_json::Value =
                serde_json::from_str(&workspace_settings_raw).unwrap_or_default();
            settings_map["language"] = serde_json::json!((*language.read()).clone());
            settings_map["timezone"] = serde_json::json!((*timezone.read()).clone());
            settings_map["week_start"] = serde_json::json!(*week_start.read());
            settings_map["template"] = serde_json::json!((*template.read()).clone());
            settings_map["grading_system"] = serde_json::json!((*grading_system.read()).clone());
            settings_map["late_policy"] = serde_json::json!((*late_policy.read()).clone());
            settings_map["target_region"] = serde_json::json!((*target_region.read()).clone());
            settings_map["moving_base_rate_per_m3"] = serde_json::json!(*base_rate.read());
            settings_map["moving_distance_fee_flat"] = serde_json::json!(*distance_fee.read());
            settings_map["moving_stairs_surcharge_per_floor"] = serde_json::json!(*stairs_surcharge.read());
            settings_map["moving_packing_supplies_fee_per_m3"] = serde_json::json!(*packing_fee.read());

            let settings_str = serde_json::to_string(&settings_map).unwrap_or_default();
            let ws_id = workspace_id.clone();
            let requester_uid = user_id.clone();
            spawn(async move {
                let _ =
                    yntra_core::update_workspace_settings(requester_uid, ws_id, settings_str).await;
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
                                        crate::components::Input {
                                            id: "org-name",
                                            class: "h-11 rounded-xl border-border/40 bg-background/40 transition-all duration-300 focus:ring-2 focus:ring-primary/20",
                                            value: "{settings_name}",
                                            oninput: move |e: FormEvent| settings_name.set(e.value()),
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
                            crate::components::Select {
                                trigger_class: "h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{language}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |val: String| {
                                        let mut save_settings = save_settings.clone();
                                        language.set(val.clone());
                                        save_settings();
                                    }
                                },
                                options: vec![
                                    ("sv".to_string(), t("settings-languages-sv", &props.locale)),
                                    ("no".to_string(), t("settings-languages-no", &props.locale)),
                                    ("da".to_string(), t("settings-languages-da", &props.locale)),
                                    ("fi".to_string(), t("settings-languages-fi", &props.locale)),
                                    ("en".to_string(), t("settings-languages-en", &props.locale)),
                                ],
                            }
                        }

                        // Target Region Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "Target Region (Compliance)"
                            }
                            crate::components::Select {
                                trigger_class: "h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{target_region}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |val: String| {
                                        let mut save_settings = save_settings.clone();
                                        target_region.set(val.clone());
                                        save_settings();
                                    }
                                },
                                options: vec![
                                    ("EU".to_string(), "EU (Default)".to_string()),
                                    ("SE".to_string(), "Sweden (SE)".to_string()),
                                    ("NO".to_string(), "Norway (NO)".to_string()),
                                    ("DK".to_string(), "Denmark (DK)".to_string()),
                                    ("FI".to_string(), "Finland (FI)".to_string()),
                                    ("US-FED".to_string(), "US Federal (FLSA)".to_string()),
                                    ("US-CA".to_string(), "US California".to_string()),
                                    ("US-CO".to_string(), "US Colorado".to_string()),
                                    ("US-NV".to_string(), "US Nevada".to_string()),
                                    ("US-AK".to_string(), "US Alaska".to_string()),
                                ],
                            }
                        }

                        // Timezone Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "{t(\"settings-timezone\", &props.locale)}"
                            }
                            crate::components::Select {
                                trigger_class: "h-11 rounded-xl border-border/40 bg-background/40",
                                value: "{timezone}",
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |val: String| {
                                        let mut save_settings = save_settings.clone();
                                        timezone.set(val.clone());
                                        save_settings();
                                    }
                                },
                                options: vec![
                                    ("Europe/Stockholm".to_string(), t("settings-timezones-stockholm", &props.locale)),
                                    ("UTC".to_string(), t("settings-timezones-utc", &props.locale)),
                                ],
                            }
                        }

                        // Week Start Dropdown
                        div { class: "space-y-2",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "{t(\"settings-week-start\", &props.locale)}"
                            }
                            crate::components::Select {
                                trigger_class: "h-11 rounded-xl border-border/40 bg-background/40",
                                value: week_start.read().to_string(),
                                onchange: {
                                    let save_settings = save_settings.clone();
                                    move |val: String| {
                                        let mut save_settings = save_settings.clone();
                                        if let Ok(parsed) = val.parse::<i32>() {
                                            week_start.set(parsed);
                                            save_settings();
                                        }
                                    }
                                },
                                options: vec![
                                    ("1".to_string(), t("settings-monday", &props.locale)),
                                    ("0".to_string(), t("settings-sunday", &props.locale)),
                                ],
                            }
                        }

                        // Grading System (only shown if school module is active)
                        if is_school {
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Grading System / Scale"
                                }
                                crate::components::Select {
                                    trigger_class: "h-11 rounded-xl border-border/40 bg-background/40",
                                    value: "{grading_system}",
                                    onchange: {
                                        let save_settings = save_settings.clone();
                                        move |val: String| {
                                            let mut save_settings = save_settings.clone();
                                            grading_system.set(val.clone());
                                            save_settings();
                                        }
                                    },
                                    options: vec![
                                        ("A-F".to_string(), "A-F (Letter Grades)".to_string()),
                                        ("1-10".to_string(), "1-10 (Numeric Scale)".to_string()),
                                        ("1-100".to_string(), "0-100 (Percentage Scale)".to_string()),
                                        ("U-G-VG".to_string(), "U, G, VG (Swedish University Scale)".to_string()),
                                        ("U-G".to_string(), "U, G (Swedish Pass/Fail)".to_string()),
                                        ("U-3-4-5".to_string(), "U, 3, 4, 5 (Swedish Engineering)".to_string()),
                                    ],
                                }
                            }
                        }

                        // Late Submission Policy (only shown if school module is active)
                        if is_school {
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Default Late Submission Policy"
                                }
                                crate::components::Select {
                                    trigger_class: "h-11 rounded-xl border-border/40 bg-background/40",
                                    value: "{late_policy}",
                                    onchange: {
                                        let save_settings = save_settings.clone();
                                        move |val: String| {
                                            let mut save_settings = save_settings.clone();
                                            late_policy.set(val.clone());
                                            save_settings();
                                        }
                                    },
                                    options: vec![
                                        ("none".to_string(), "None (No Penalties)".to_string()),
                                        ("hard_deadline".to_string(), "Hard Deadline (Block Late Submissions)".to_string()),
                                        ("penalty_5".to_string(), "5% Daily Deduction Penalty".to_string()),
                                        ("penalty_10".to_string(), "10% Daily Deduction Penalty".to_string()),
                                    ],
                                }
                            }
                        }
                    }
                }

                if *template.read() == "moving" {
                    components::Card { class: "flex h-full flex-col overflow-hidden border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-md",
                        components::CardHeader { class: "pb-4",
                            div { class: "flex items-center gap-3",
                                div { class: "rounded-xl bg-primary/10 p-2.5 text-primary shadow-inner",
                                    components::LucideIcon { name: "calculator", class: "h-5 w-5" }
                                }
                                div {
                                    components::CardTitle { class: "text-lg font-bold tracking-tight",
                                        "Moving Pricing Rates"
                                    }
                                    components::CardDescription { class: "text-xs",
                                        "Configure hourly base rates, stair fees, and packaging costs."
                                    }
                                }
                            }
                        }
                        components::CardContent { class: "grid flex-1 grid-cols-1 gap-4 sm:grid-cols-2",
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Base Labor Rate (per m³)"
                                }
                                crate::components::Input {
                                    class: "h-11 rounded-xl border-border/40 bg-background/40",
                                    r#type: "number".to_string(),
                                    value: base_rate.read().to_string(),
                                    oninput: move |e: FormEvent| {
                                        if let Ok(parsed) = e.value().parse::<f64>() {
                                            base_rate.set(parsed);
                                        }
                                    },
                                    onblur: {
                                        let mut save = save_settings.clone();
                                        move |_| save()
                                    }
                                }
                            }
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Flat Distance Fee (SEK)"
                                }
                                crate::components::Input {
                                    class: "h-11 rounded-xl border-border/40 bg-background/40",
                                    r#type: "number".to_string(),
                                    value: distance_fee.read().to_string(),
                                    oninput: move |e: FormEvent| {
                                        if let Ok(parsed) = e.value().parse::<f64>() {
                                            distance_fee.set(parsed);
                                        }
                                    },
                                    onblur: {
                                        let mut save = save_settings.clone();
                                        move |_| save()
                                    }
                                }
                            }
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Stairs Surcharge (per floor)"
                                }
                                crate::components::Input {
                                    class: "h-11 rounded-xl border-border/40 bg-background/40",
                                    r#type: "number".to_string(),
                                    value: stairs_surcharge.read().to_string(),
                                    oninput: move |e: FormEvent| {
                                        if let Ok(parsed) = e.value().parse::<f64>() {
                                            stairs_surcharge.set(parsed);
                                        }
                                    },
                                    onblur: {
                                        let mut save = save_settings.clone();
                                        move |_| save()
                                    }
                                }
                            }
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Packing Supplies (per m³)"
                                }
                                crate::components::Input {
                                    class: "h-11 rounded-xl border-border/40 bg-background/40",
                                    r#type: "number".to_string(),
                                    value: packing_fee.read().to_string(),
                                    oninput: move |e: FormEvent| {
                                        if let Ok(parsed) = e.value().parse::<f64>() {
                                            packing_fee.set(parsed);
                                        }
                                    },
                                    onblur: {
                                        let mut save = save_settings.clone();
                                        move |_| save()
                                    }
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
