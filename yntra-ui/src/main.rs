#![allow(clippy::collapsible_if)]
// Trigger rebuild to pick up tailwind.css changes
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;

pub mod components;
pub mod locales;
pub mod utils;
pub mod views;
pub mod blocks;
pub mod state;
pub mod layouts;

fn main() {
    // Initialize the logger
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");

    // Launch the Dioxus App
    #[cfg(not(target_arch = "wasm32"))]
    {
        let config = dioxus::desktop::Config::new().with_window(
            dioxus::desktop::WindowBuilder::new()
                .with_always_on_top(false)
        );
        dioxus::LaunchBuilder::new()
            .with_cfg(config)
            .launch(App);
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

    let selected_note_team_id = state.selected_note_team_id;
    let active_message_id = state.active_message_id;
    let globalsearch_open = state.globalsearch_open;
    let settings_tab = state.settings_tab;
    let report_tab = state.report_tab;
    let report_type = state.report_type;
    let db_trigger = state.db_trigger;
    let account_preferences = state.account_preferences;

    // Handle section changes with guards
    let active_user = state.users.read().as_ref().and_then(|u_list| u_list.iter().find(|u| u.id == *active_user_id.read()).cloned()).unwrap_or_else(|| WorkspaceUser {
        id: String::new(),
        workspace_id: None,
        email: String::new(),
        full_name: Some("Guest User".to_string()),
        phone: None,
        role: "guest".to_string(),
        preferences: "{}".to_string(),
        siths_card_id: None,
        nfc_badge_uid: None,
        updated_at: 0,
        sync_status: "synced".to_string(),
        personal_number: None,
    });
    let current_role = active_user.role.clone();
    let is_client = current_role == "client";

    if is_client && *active_section.read() != "messaging" && *active_section.read() != "client_portal" && *active_section.read() != "directory"
    {
        active_section.set("client_portal".to_string());
    }

    let workspace_val = state.workspace.read().clone().unwrap_or_else(|| yntra_core::Workspace {
        id: "workspace-1".to_string(),
        name: "Yntra Operations Ltd".to_string(),
        modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
        settings: "{}".to_string(),
        brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
        logo_url: None,
        block_settings: "{}".to_string(),
    });
    
    let users_data = state.users.read().clone().unwrap_or_default();
    let teams_data = state.teams.read().clone().unwrap_or_default();
    let notes_data = state.notes.read().clone().unwrap_or_default();
    let messages_data = state.messages.read().clone().unwrap_or_default();

    let on_desktop_oauth_callback = state.on_desktop_oauth.clone();

    rsx! {
        // Load DX Components Theme
        style { {include_str!("../public/dx-components-theme.css")} }
        // Load compiled Tailwind CSS
        style { {include_str!("../public/tailwind.css")} }
        // Load extracted global CSS stylesheet
        style { {include_str!("../public/global.css")} }
        // Load SQLite Web Worker Bridge
        script { src: "/db-bridge.js" }

        components::ToastProvider {
            BackgroundErrorListener {}
            components::VisualEffectHandler {
                account_preferences,
                workspace_brand_color: workspace_val.brand_color.clone(),
            }
            components::OfflineIndicator {}
            components::GlobalSearch {
                open: globalsearch_open,
                active_section,
                settings_tab,
                report_tab,
                report_type,
                selected_note_team_id,
                active_message_id,
                users: users_data,
                teams: teams_data,
                notes: notes_data,
                messages: messages_data,
            }

            if !*logged_in.read() {
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
                    users: state.users.read().clone().unwrap_or_default(),
                    db_trigger: db_trigger,
                    two_factor_user: two_factor_user,
                    on_desktop_oauth: move |provider| on_desktop_oauth_callback.call(provider),
                }
            } else if state.users.read().is_none() || state.workspace.read().is_none() {
                div {
                    class: "flex h-screen w-screen items-center justify-center bg-gray-50 dark:bg-zinc-900",
                    div {
                        class: "flex flex-col items-center space-y-4",
                        div { class: "h-12 w-12 animate-spin rounded-full border-4 border-blue-500 border-t-transparent" }
                        p { class: "text-gray-500 dark:text-zinc-400 font-medium", "Laddar Yntra..." }
                    }
                }
            } else if *needs_setup.read() {
                views::SetupView {
                    active_user_id: active_user_id,
                    needs_setup: needs_setup,
                    logged_in: logged_in,
                    auth_region: auth_region,
                    db_trigger: db_trigger,
                    users: state.users.read().clone().unwrap_or_default(),
                }
            } else if is_client {
                layouts::ClientLayout {}
            } else {
                layouts::EmployeeLayout {}
            }
        }
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
