use super::TwoFactorSettings;
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use yntra_core::update_user_profile;

#[derive(Props, Clone)]
pub struct AccountSettingsProps {
    pub active_user: WorkspaceUser,
    pub account_name: Signal<String>,
    pub account_phone: Signal<String>,
    pub account_preferences: Signal<String>,
    pub account_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for AccountSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

const ACCENT_COLORS: &[(&str, &str)] = &[
    ("Yntra Blue", "primary"),
    ("Azure", "#3b82f6"),
    ("Emerald", "#10b981"),
    ("Violet", "#8b5cf6"),
    ("Amber", "#f59e0b"),
    ("Crimson", "#ef4444"),
    ("Slate", "#64748b"),
    ("Midnight", "#0f172a"),
];

#[component]
pub fn AccountSettings(props: AccountSettingsProps) -> Element {
    let active_user = props.active_user.clone();
    let mut account_name = props.account_name;
    let mut account_phone = props.account_phone;
    let mut account_preferences = props.account_preferences;
    let mut account_save_status = props.account_save_status;
    let mut db_trigger = props.db_trigger;

    // Local state loaded from preferences JSON in the local SQLite database
    let mut phone_privacy = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("phone_privacy")
            .and_then(|p| p.as_str())
            .unwrap_or("organization")
            .to_string()
    });
    let mut location_privacy = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("location_privacy")
            .and_then(|p| p.as_str())
            .unwrap_or("organization")
            .to_string()
    });
    let mut location_val = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("location")
            .and_then(|l| l.as_str())
            .unwrap_or("Stockholm, SE")
            .to_string()
    });
    let mut selected_accent = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("accent")
            .and_then(|a| a.as_str())
            .unwrap_or("primary")
            .to_string()
    });
    let mut selected_theme = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("theme")
            .and_then(|t| t.as_str())
            .unwrap_or("dark")
            .to_string()
    });
    // In React it is a float like 1.0. Let's keep it as f32 in range [0.8, 1.2]
    let mut font_scale = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        let scale = prefs
            .get("font_scale")
            .and_then(|f| f.as_f64())
            .unwrap_or(1.0) as f32;
        if scale > 2.0 { scale / 100.0 } else { scale }
    });
    let mut selected_language = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("language")
            .and_then(|l| l.as_str())
            .unwrap_or("US")
            .to_string()
    });

    let update_preference = {
        let active_user_id = active_user.id.clone();
        let account_name = account_name;
        let account_phone = account_phone;

        move |key: &str, val: serde_json::Value| {
            account_save_status.set("saving".to_string());
            let mut prefs: serde_json::Value =
                serde_json::from_str(&account_preferences.read()).unwrap_or_default();
            prefs[key] = val.clone();
            if key == "accent" {
                prefs["accent_color"] = val;
            }
            let prefs_str = serde_json::to_string(&prefs).unwrap_or_default();
            account_preferences.set(prefs_str.clone());

            let uid = active_user_id.clone();
            let name_val = Some((*account_name.read()).clone());
            let phone_val = Some((*account_phone.read()).clone());
            let prefs_val = prefs_str.clone();
            let requester_uid = uid.clone();
            spawn(async move {
                let _ = update_user_profile(
                    requester_uid.clone(),
                    requester_uid,
                    name_val,
                    phone_val,
                    prefs_val,
                )
                .await;
            });

            let current_trig = *db_trigger.read();
            db_trigger.set(current_trig + 1);
            account_save_status.set("saved".to_string());
        }
    };

    let handle_update_profile = {
        let active_user_id = active_user.id.clone();
        let account_name = account_name;
        let account_phone = account_phone;

        move || {
            account_save_status.set("saving".to_string());

            let mut prefs: serde_json::Value =
                serde_json::from_str(&account_preferences.read()).unwrap_or_default();
            prefs["phone_privacy"] = serde_json::json!(*phone_privacy.read());
            prefs["location_privacy"] = serde_json::json!(*location_privacy.read());
            prefs["location"] = serde_json::json!(*location_val.read());

            let prefs_str = serde_json::to_string(&prefs).unwrap_or_default();
            account_preferences.set(prefs_str.clone());

            let uid = active_user_id.clone();
            let name_val = Some((*account_name.read()).clone());
            let phone_val = Some((*account_phone.read()).clone());
            let prefs_val = prefs_str.clone();
            spawn(async move {
                let _ = update_user_profile(uid.clone(), uid, name_val, phone_val, prefs_val).await;
            });

            let current_trig = *db_trigger.read();
            db_trigger.set(current_trig + 1);
            account_save_status.set("saved".to_string());
        }
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "grid grid-cols-1 gap-6 md:grid-cols-2",

                // 1. Personal Details Card
                components::Card { class: "flex flex-col border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-sm",
                    components::CardHeader {
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-lg bg-primary/10 p-2 text-primary",
                                components::LucideIcon { name: "user", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg", "{t(\"settings-account-profile\", &props.locale)}" }
                                components::CardDescription { "{t(\"settings-account-profile-desc\", &props.locale)}" }
                            }
                        }
                    }
                    components::CardContent { class: "flex-1 space-y-4",
                        // Email (disabled)
                        div { class: "space-y-2",
                            label {
                                r#for: "email",
                                class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                "Email Address"
                            }
                            input {
                                id: "email",
                                class: "yntra-input h-11 border-border/50 bg-muted/50",
                                value: "{active_user.email}",
                                disabled: true,
                                style: "opacity:0.6; cursor:not-allowed;"
                            }
                            p { class: "text-[10px] text-muted-foreground m-0",
                                "{t(\"settings-account-email-change-info\", &props.locale)}"
                            }
                        }

                        // Full Name
                        div { class: "space-y-2",
                            label {
                                r#for: "name",
                                class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                "Name"
                            }
                            input {
                                id: "name",
                                class: "yntra-input h-11 border-border/50 bg-background/50",
                                value: "{account_name}",
                                oninput: move |e| account_name.set(e.value())
                            }
                        }

                        // Phone & Privacy
                        div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                            div { class: "space-y-2",
                                label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                    "{t(\"settings-account-phone\", &props.locale)}"
                                }
                                div { class: "space-y-1.5",
                                    input {
                                        class: "yntra-input h-11 border-border/50 bg-background/50",
                                        value: "{account_phone}",
                                        placeholder: "+46...",
                                        oninput: move |e| account_phone.set(e.value())
                                    }
                                    select {
                                        class: "yntra-input h-8 border-border/40 bg-secondary/30 text-[11px] p-0 px-2",
                                        value: "{phone_privacy}",
                                        onchange: move |e| phone_privacy.set(e.value()),
                                        option { value: "everyone", "{t(\"settings-account-visibility-everyone\", &props.locale)}" }
                                        option { value: "organization", "{t(\"settings-account-visibility-organization\", &props.locale)}" }
                                        option { value: "none", "{t(\"settings-account-visibility-none\", &props.locale)}" }
                                    }
                                }
                            }

                            // Location & Privacy
                            div { class: "space-y-2",
                                label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                    "{t(\"settings-account-location\", &props.locale)}"
                                }
                                div { class: "space-y-1.5",
                                    input {
                                        class: "yntra-input h-11 border-border/50 bg-background/50",
                                        value: "{location_val}",
                                        placeholder: "Stockholm, SE",
                                        oninput: move |e| location_val.set(e.value())
                                    }
                                    select {
                                        class: "yntra-input h-8 border-border/40 bg-secondary/30 text-[11px] p-0 px-2",
                                        value: "{location_privacy}",
                                        onchange: move |e| location_privacy.set(e.value()),
                                        option { value: "everyone", "{t(\"settings-account-visibility-everyone\", &props.locale)}" }
                                        option { value: "organization", "{t(\"settings-account-visibility-organization\", &props.locale)}" }
                                        option { value: "none", "{t(\"settings-account-visibility-none\", &props.locale)}" }
                                    }
                                }
                            }
                        }

                        div { class: "mt-auto pt-4",
                            components::Button {
                                class: "w-full shadow-lg shadow-primary/20",
                                onclick: {
                                    let mut handle_update_profile = handle_update_profile.clone();
                                    move |_| handle_update_profile()
                                },
                                span { "Save Profile Settings" }
                            }
                        }
                    }
                }

                // 2. Visual Customization Card
                components::Card { class: "border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-sm",
                    components::CardHeader {
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-lg bg-primary/10 p-2 text-primary",
                                components::LucideIcon { name: "palette", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg", "{t(\"settings-account-appearance\", &props.locale)}" }
                                components::CardDescription { "{t(\"settings-account-appearance-desc\", &props.locale)}" }
                            }
                        }
                    }
                    components::CardContent { class: "space-y-8",

                        // Accent Color Scheme
                        div { class: "space-y-4",
                            label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                "{t(\"settings-account-accent-color\", &props.locale)}"
                            }
                            div { class: "grid grid-cols-2 gap-3 sm:grid-cols-4",
                                {
                                    ACCENT_COLORS.iter().map(|&(c_name, c_val)| {
                                        let is_active = *selected_accent.read() == c_val;
                                        let bg_val = if c_val == "primary" { "var(--accent-color)" } else { c_val };

                                        rsx! {
                                            button {
                                                key: "{c_val}",
                                                class: if is_active {
                                                    "group relative flex items-center gap-3 rounded-2xl border-2 p-3 text-left transition-all duration-300 hover:scale-[1.02] active:scale-95 border-primary bg-primary/5 shadow-md shadow-primary/5"
                                                } else {
                                                    "group relative flex items-center gap-3 rounded-2xl border-2 p-3 text-left transition-all duration-300 hover:scale-[1.02] active:scale-95 border-border/40 bg-background/40 hover:border-primary/40 hover:bg-background/60"
                                                },
                                                onclick: {
                                                    let mut update_preference = update_preference.clone();
                                                    move |_| {
                                                        selected_accent.set(c_val.to_string());
                                                        update_preference("accent", serde_json::json!(c_val));
                                                    }
                                                },
                                                div {
                                                    class: "h-8 w-8 flex-shrink-0 rounded-xl border border-black/10 shadow-inner",
                                                    style: "background-color: {bg_val};"
                                                }
                                                div { class: "min-w-0 flex-1",
                                                    p {
                                                        class: if is_active {
                                                            "truncate text-[11px] font-bold text-primary m-0"
                                                        } else {
                                                            "truncate text-[11px] font-bold text-foreground m-0"
                                                        },
                                                        "{c_name}"
                                                    }
                                                }
                                                if is_active {
                                                    div { class: "absolute -right-2 -top-2 rounded-full bg-primary p-1 text-primary-foreground shadow-lg ring-2 ring-background duration-300 animate-in zoom-in flex items-center justify-center",
                                                        components::LucideIcon { name: "check", class: "h-3 w-3" }
                                                    }
                                                }
                                            }
                                        }
                                    })
                                }
                            }
                        }

                        // Custom & Org Color Section
                        div { class: "space-y-4 border-t border-border/40 pt-6",
                            div { class: "flex items-center justify-between",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "Custom Branding"
                                }
                                div { class: "flex items-center gap-1.5 rounded-full bg-primary/10 px-2 py-0.5 text-[9px] font-bold uppercase tracking-wider text-primary",
                                    "Premium"
                                }
                            }

                            div { class: "group/custom flex flex-col items-center gap-4 rounded-3xl border border-border/40 bg-muted/20 p-4 backdrop-blur-sm sm:flex-row",
                                components::HexColorPicker {
                                    value: {
                                        let val = selected_accent.read().clone();
                                        if val.starts_with('#') { val } else { "#3b82f6".to_string() }
                                    },
                                    onchange: {
                                        let mut update_preference = update_preference.clone();
                                        move |color: String| {
                                            selected_accent.set(color.clone());
                                            update_preference("accent", serde_json::json!(color));
                                        }
                                    }
                                }

                                div { class: "flex-1 space-y-1 text-center sm:text-left",
                                    p { class: "text-[11px] font-bold uppercase tracking-widest text-foreground m-0",
                                        "{t(\"settings-account-custom-color-title\", &props.locale)}"
                                    }
                                    p { class: "text-[10px] text-muted-foreground/70 m-0",
                                        "{t(\"settings-account-custom-color-desc\", &props.locale)}"
                                    }
                                }

                                div { class: "flex items-center gap-2 rounded-xl border border-border/50 bg-background/60 p-1.5",
                                    span { class: "px-2 font-mono text-[10px] font-bold uppercase text-muted-foreground/80",
                                        "HEX"
                                    }
                                    input {
                                        class: "h-8 w-24 border-none bg-transparent text-right font-mono text-[11px] font-bold focus-visible:ring-0 outline-none text-foreground",
                                        value: {
                                            let val = selected_accent.read().clone();
                                            if val == "primary" { "#3B82F6".to_string() } else { val }
                                        },
                                        oninput: {
                                            let mut update_preference = update_preference.clone();
                                            move |e| {
                                                let val = e.value();
                                                if val.starts_with('#') && val.len() <= 7 {
                                                    selected_accent.set(val.clone());
                                                    update_preference("accent", serde_json::json!(val));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Theme & Scale Section
                        div { class: "space-y-6 border-t border-border/40 pt-6",
                            div { class: "space-y-4",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "{t(\"settings-choose-theme\", &props.locale)}"
                                }
                                div { class: "grid grid-cols-2 gap-3 sm:grid-cols-3",
                                    {
                                        vec![
                                            ("light", "sun", t("settings-theme-light", &props.locale)),
                                            ("dark", "moon", t("settings-theme-dark", &props.locale)),
                                            ("midnight", "zap", "Midnight".to_string()),
                                            ("slate", "shield", "Slate".to_string()),
                                            ("forest", "leaf", "Forest".to_string()),
                                            ("system", "monitor", t("settings-theme-system", &props.locale)),
                                        ].into_iter().map(|(val, icon, label)| {
                                            let is_active = *selected_theme.read() == val;

                                            rsx! {
                                                button {
                                                    key: "{val}",
                                                    class: if is_active {
                                                        "flex flex-col items-center justify-center rounded-2xl border-2 p-3 transition-all duration-300 border-primary bg-primary/10 text-primary shadow-lg shadow-primary/5"
                                                    } else {
                                                        "flex flex-col items-center justify-center rounded-2xl border-2 p-3 transition-all duration-300 border-border/40 bg-background/40 text-muted-foreground hover:border-primary/40 hover:text-foreground"
                                                    },
                                                    onclick: {
                                                        let mut update_preference = update_preference.clone();
                                                        move |_| {
                                                            selected_theme.set(val.to_string());
                                                            update_preference("theme", serde_json::json!(val));
                                                        }
                                                    },
                                                    components::LucideIcon { name: icon, class: "mb-1.5 h-5 w-5" }
                                                    span { class: "text-[10px] font-bold uppercase tracking-wider", "{label}" }
                                                }
                                            }
                                        })
                                    }
                                }
                            }

                            // Font Scale Slider
                            div { class: "space-y-4 pt-2",
                                div { class: "flex justify-between items-center",
                                    label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                        "{t(\"settings-font-scale\", &props.locale)}"
                                    }
                                    span { class: "rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-bold text-primary shadow-inner",
                                        "{(*font_scale.read() * 100.0) as i32}%"
                                    }
                                }
                                input {
                                    r#type: "range",
                                    min: "0.8",
                                    max: "1.2",
                                    step: "0.05",
                                    class: "w-full cursor-pointer",
                                    value: "{font_scale}",
                                    oninput: {
                                        let mut update_preference = update_preference.clone();
                                        move |e| {
                                            if let Ok(val) = e.value().parse::<f32>() {
                                                font_scale.set(val);
                                                update_preference("font_scale", serde_json::json!(val));
                                            }
                                        }
                                    }
                                }
                            }

                            // Preferred Language dropdown
                            div { class: "space-y-4 border-t border-border/40 pt-6",
                                label { class: "text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/70",
                                    "{t(\"settings-language\", &props.locale)}"
                                }
                                select {
                                    class: "yntra-input h-11 rounded-xl border-border/40 bg-background/40 w-full",
                                    value: "{selected_language}",
                                    onchange: {
                                        let mut update_preference = update_preference.clone();
                                        move |e| {
                                            let val = e.value();
                                            selected_language.set(val.clone());
                                            update_preference("language", serde_json::json!(val));
                                        }
                                    },
                                    option { value: "sv", "{t(\"settings-languages-sv\", &props.locale)}" }
                                    option { value: "no", "{t(\"settings-languages-no\", &props.locale)}" }
                                    option { value: "da", "{t(\"settings-languages-da\", &props.locale)}" }
                                    option { value: "fi", "{t(\"settings-languages-fi\", &props.locale)}" }
                                    option { value: "en", "{t(\"settings-languages-en\", &props.locale)}" }
                                }
                            }
                        }
                    }
                }
            }

            // 3. Two-Factor Authentication Card (at the bottom)
            components::Card { class: "border-amber-500/20 bg-amber-500/[0.03] shadow-none",
                components::CardContent { class: "space-y-4 p-4",
                    TwoFactorSettings {
                        active_user: active_user.clone(),
                        account_preferences: props.account_preferences,
                        account_save_status: props.account_save_status,
                        db_trigger: props.db_trigger,
                        locale: props.locale.clone(),
                    }
                    div { class: "flex items-center gap-2 border-t border-amber-500/10 pt-2 text-[10px] font-medium italic text-amber-600/60",
                        components::LucideIcon { name: "clock", class: "h-3 w-3" }
                        span { "{t(\"common-last-active\", &props.locale)}: -" }
                    }
                }
            }
        }
    }
}
