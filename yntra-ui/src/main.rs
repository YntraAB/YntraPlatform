#![allow(clippy::collapsible_if)]
// Trigger rebuild to pick up tailwind.css changes
use dioxus::prelude::*;

pub mod blocks;
pub mod components;
pub mod layouts;
pub mod locales;
pub mod state;
pub mod utils;
pub mod views;

fn main() {
    // Initialize the logger
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");

    // Launch the Dioxus App
    #[cfg(not(target_arch = "wasm32"))]
    {
        let config = dioxus::desktop::Config::new()
            .with_window(dioxus::desktop::WindowBuilder::new().with_always_on_top(false));
        dioxus::LaunchBuilder::new().with_cfg(config).launch(App);
    }

    #[cfg(target_arch = "wasm32")]
    {
        dioxus::launch(App);
    }
}

#[component]
fn App() -> Element {
    // 1. Initialize the global app state
    let state = state::use_init_app_state();
    provide_context(state);

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        spawn(async move {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                if let Ok(storage) = js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("storage")) {
                    if !storage.is_undefined() {
                        use wasm_bindgen::JsCast;
                        let storage_mgr: web_sys::StorageManager = storage.unchecked_into();
                        let _ = storage_mgr.persist();
                    }
                }
            }
        });
    });

    rsx! {
        Stylesheet {}
        document::Link { rel: "manifest", href: asset!("/public/manifest.json") }
        // Load SQLite Web Worker Bridge, PWA Service Worker, & Sentry Telemetry
        script { src: asset!("/public/db-bridge.js") }
        script { src: asset!("/public/sw-register.js") }
        script { src: asset!("/public/sentry-init.js") }

        components::ToastProvider {
            BackgroundErrorListener {}
            VisualEffectContainer {}
            components::OfflineIndicator {}
            components::AutoUpdateToast {}
            components::TelemetryObserver {}
            GlobalSearchContainer {}
            MainContent {}
        }
    }
}

#[component]
fn VisualEffectContainer() -> Element {
    let state = use_context::<state::AppState>();
    let workspace_val = state.workspace.read().clone().unwrap_or_else(|| yntra_core::Workspace {
        id: "workspace-1".to_string(),
        name: "Yntra Operations Ltd".to_string(),
        modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
        settings: "{}".to_string(),
        brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
        logo_url: None,
        block_settings: "{}".to_string(),
        updated_at: 0,
        sync_status: "synced".to_string(),
    });

    rsx! {
        components::VisualEffectHandler {
            account_preferences: state.account_preferences,
            workspace_brand_color: workspace_val.brand_color,
        }
    }
}

#[component]
fn GlobalSearchContainer() -> Element {
    let state = use_context::<state::AppState>();
    rsx! {
        components::GlobalSearch {
            open: state.globalsearch_open,
            active_section: state.active_section,
            settings_tab: state.settings_tab,
            report_tab: state.report_tab,
            report_type: state.report_type,
            selected_note_team_id: state.selected_note_team_id,
            active_message_id: state.active_message_id,
        }
    }
}

#[component]
fn MainContent() -> Element {
    let state = use_context::<state::AppState>();

    let logged_in = state.logged_in;
    let active_user_id = state.active_user_id;
    let mut active_section = state.active_section;
    let needs_setup = state.needs_setup;
    let two_factor_user = state.two_factor_user;

    // Login & Auth State Signals
    let login_tab = state.login_tab;
    let show_bankid_modal = state.show_bankid_modal;
    let scanning_state = state.scanning_state;
    let login_email = state.login_email;
    let login_password = state.login_password;
    let login_error = state.login_error;
    let auth_region = state.auth_region;
    let db_trigger = state.db_trigger;

    let active_role = state.active_user_role;
    let is_client = *active_role.read() == "client";

    use_effect(move || {
        let role = active_role.read().clone();
        let current_sec = active_section.read().clone();
        let is_c = role == "client";
        let is_s = role == "student" || role == "role-school-student";
        let is_p = role == "parent" || role == "role-school-parent";

        if is_c
            && current_sec != "messaging"
            && current_sec != "client_portal"
            && current_sec != "directory"
        {
            active_section.set("client_portal".to_string());
        }

        if is_s
            && current_sec != "academics"
            && current_sec != "report_cards"
            && current_sec != "library"
            && current_sec != "finance"
            && current_sec != "health_clinic"
            && current_sec != "dashboard"
            && current_sec != "settings"
        {
            active_section.set("academics".to_string());
        }

        if is_p
            && current_sec != "academics"
            && current_sec != "finance"
            && current_sec != "library"
            && current_sec != "messaging"
            && current_sec != "directory"
            && current_sec != "dashboard"
            && current_sec != "settings"
        {
            active_section.set("academics".to_string());
        }
    });

    let workspace_val = state.workspace.read().clone().unwrap_or_else(|| yntra_core::Workspace {
        id: "workspace-1".to_string(),
        name: "Yntra Operations Ltd".to_string(),
        modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
        settings: "{}".to_string(),
        brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
        logo_url: None,
        block_settings: "{}".to_string(),
        updated_at: 0,
        sync_status: "synced".to_string(),
    });

    let on_desktop_oauth_callback = state.on_desktop_oauth.clone();
    let users_val = state.users.read().clone().unwrap_or_default();
    let is_users_none = state.users.read().is_none();
    let is_workspace_none = state.workspace.read().is_none();

    if !*logged_in.read() {
        rsx! {
            views::LoginView {
                scanning_state: scanning_state,
                show_bankid_modal: show_bankid_modal,
                login_error: login_error,
                login_tab: login_tab,
                auth_region: auth_region,
                logged_in: logged_in,
                active_user_id: active_user_id,
                active_section: active_section,
                login_email: login_email,
                login_password: login_password,
                workspace: workspace_val.clone(),
                needs_setup: needs_setup,
                users: users_val,
                db_trigger: db_trigger,
                two_factor_user: two_factor_user,
                on_desktop_oauth: move |provider| on_desktop_oauth_callback.call(provider),
            }
        }
    } else if is_users_none || is_workspace_none {
        rsx! {
            div {
                class: "flex h-screen w-screen items-center justify-center bg-gray-50 dark:bg-zinc-900",
                div {
                    class: "flex flex-col items-center space-y-4",
                    div { class: "h-12 w-12 animate-spin rounded-full border-4 border-blue-500 border-t-transparent" }
                    p { class: "text-gray-500 dark:text-zinc-400 font-medium", "Laddar Yntra..." }
                }
            }
        }
    } else if *needs_setup.read() {
        rsx! {
            views::SetupView {
                active_user_id: active_user_id,
                needs_setup: needs_setup,
                logged_in: logged_in,
                auth_region: auth_region,
                db_trigger: db_trigger,
                users: users_val,
            }
        }
    } else if is_client {
        rsx! { layouts::ClientLayout {} }
    } else {
        rsx! { layouts::EmployeeLayout {} }
    }
}

#[component]
fn BackgroundErrorListener() -> Element {
    let state = use_context::<state::AppState>();
    let toast = dioxus_primitives::toast::use_toast();
    let mut bg_err = state.background_error;

    use_effect(move || {
        if let Some(err) = bg_err.read().as_ref() {
            let user_err = crate::utils::map_error(err);
            toast.error(
                user_err.title,
                dioxus_primitives::toast::ToastOptions::new().description(user_err.description),
            );
            // Clear the error so it doesn't fire repeatedly
            spawn(async move {
                bg_err.set(None);
            });
        }
    });

    rsx! {}
}

#[component]
fn Stylesheet() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/public/dx-components-theme.css") }
        document::Link { rel: "stylesheet", href: asset!("/public/tailwind.css") }
        document::Link { rel: "stylesheet", href: asset!("/public/global.css") }
    }
}
