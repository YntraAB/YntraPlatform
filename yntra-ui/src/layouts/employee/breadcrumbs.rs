use crate::locales::t;
use dioxus::prelude::*;

#[derive(Clone)]
pub struct BreadcrumbItem {
    pub label: String,
    pub onclick: Callback<()>,
}

impl PartialEq for BreadcrumbItem {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
    }
}

#[allow(clippy::too_many_arguments)]
pub fn get_breadcrumbs(
    active_section: Signal<String>,
    selected_note_team_id: Signal<String>,
    selected_note_id: Signal<Option<String>>,
    is_note_composing: Signal<bool>,
    active_message_id: Signal<Option<String>>,
    messaging_view_tab: Signal<String>,
    selected_directory_team: Signal<Option<String>>,
    directory_level: Signal<String>,
    selected_directory_workspace: Signal<String>,
    selected_calendar_date: Signal<String>,
    active_user_role: &str,
    locale: &str,
    teams: &[yntra_core::Team],
    notes: &[yntra_core::DailyNote],
    messages: &[yntra_core::MessageItem],
    workspaces: &[yntra_core::Workspace],
) -> Vec<BreadcrumbItem> {
    let mut list = Vec::new();

    // 1. Home
    {
        let mut active_section = active_section;
        let mut selected_note_team_id = selected_note_team_id;
        let mut selected_note_id = selected_note_id;
        let mut is_note_composing = is_note_composing;
        let mut active_message_id = active_message_id;
        let mut messaging_view_tab = messaging_view_tab;
        let mut selected_directory_team = selected_directory_team;
        let mut directory_level = directory_level;
        let locale_str = locale.to_string();
        let is_plat_admin = active_user_role == "platform_admin";

        list.push(BreadcrumbItem {
            label: t("common-home", &locale_str).to_string(),
            onclick: Callback::new(move |_| {
                active_section.set("dashboard".to_string());
                selected_note_team_id.set(String::new());
                selected_note_id.set(None);
                is_note_composing.set(false);
                active_message_id.set(None);
                messaging_view_tab.set("inbox".to_string());
                selected_directory_team.set(None);
                let lvl = if is_plat_admin {
                    "workspaces".to_string()
                } else {
                    "teams".to_string()
                };
                directory_level.set(lvl);
            }),
        });
    }

    let section = active_section.read().clone();

    // 2. Base section item
    if section != "dashboard" {
        let label = match section.as_str() {
            "notes" => t("section-notes", locale).to_string(),
            "messaging" => {
                if active_user_role == "client" {
                    "Care Chat".to_string()
                } else {
                    t("section-messaging", locale).to_string()
                }
            }
            "directory" => t("section-directory", locale).to_string(),
            "scheduling" => t("section-scheduling", locale).to_string(),
            "time" => t("section-time", locale).to_string(),
            "assistance" => t("section-assistance", locale).to_string(),
            "journals" => t("section-journals", locale).to_string(),
            "medications" => t("section-medications", locale).to_string(),
            "reporting" => t("section-reporting", locale).to_string(),
            "jobs" => t("jobs-nav", locale).to_string(),
            "dispatch" => t("section-dispatch", locale).to_string(),
            "live_map" => t("section-live-map", locale).to_string(),
            "fleet" => t("section-fleet", locale).to_string(),
            "rut_exports" => t("section-rut-exports", locale).to_string(),
            "booking_widget" => t("section-booking-widget", locale).to_string(),
            "settings" => t("common-settings", locale).to_string(),
            _ => section.clone(),
        };

        let mut active_section = active_section;
        let mut selected_note_team_id = selected_note_team_id;
        let mut selected_note_id = selected_note_id;
        let mut is_note_composing = is_note_composing;
        let mut active_message_id = active_message_id;
        let mut messaging_view_tab = messaging_view_tab;
        let mut selected_directory_team = selected_directory_team;
        let mut directory_level = directory_level;

        let section_to_set = section.clone();
        let is_plat_admin = active_user_role == "platform_admin";

        list.push(BreadcrumbItem {
            label,
            onclick: Callback::new(move |_| {
                active_section.set(section_to_set.clone());
                selected_note_team_id.set(String::new());
                selected_note_id.set(None);
                is_note_composing.set(false);
                active_message_id.set(None);
                messaging_view_tab.set("inbox".to_string());
                selected_directory_team.set(None);
                let lvl = if is_plat_admin {
                    "workspaces".to_string()
                } else {
                    "teams".to_string()
                };
                directory_level.set(lvl);
            }),
        });
    }

    // 3. Sub-paths based on active sections
    match section.as_str() {
        "notes" => {
            let team_id = selected_note_team_id.read().clone();
            if !team_id.is_empty() {
                let team_name = teams
                    .iter()
                    .find(|t| t.id == team_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| "Unknown Team".to_string());

                let mut selected_note_id = selected_note_id;
                let mut is_note_composing = is_note_composing;

                list.push(BreadcrumbItem {
                    label: team_name,
                    onclick: Callback::new(move |_| {
                        selected_note_id.set(None);
                        is_note_composing.set(false);
                    }),
                });

                if *is_note_composing.read() {
                    list.push(BreadcrumbItem {
                        label: t("notes-compose-title", locale).to_string(),
                        onclick: Callback::new(|_| {}),
                    });
                } else if let Some(ref note_id) = *selected_note_id.read() {
                    let note_subject = notes
                        .iter()
                        .find(|n| &n.id == note_id)
                        .map(|n| n.subject.clone())
                        .unwrap_or_else(|| "Untitled Note".to_string());

                    list.push(BreadcrumbItem {
                        label: note_subject,
                        onclick: Callback::new(|_| {}),
                    });
                }
            }
        }
        "messaging" => {
            let view_tab = messaging_view_tab.read().clone();
            if view_tab == "compose" {
                list.push(BreadcrumbItem {
                    label: t("messages-compose", locale).to_string(),
                    onclick: Callback::new(|_| {}),
                });
            } else if let Some(ref msg_id) = *active_message_id.read() {
                let msg_subject = messages
                    .iter()
                    .find(|m| &m.id == msg_id)
                    .and_then(|m| m.subject.clone())
                    .unwrap_or_else(|| "Message Detail".to_string());

                list.push(BreadcrumbItem {
                    label: msg_subject,
                    onclick: Callback::new(|_| {}),
                });
            }
        }
        "directory" => {
            let level = directory_level.read().clone();

            // Only push Workspace Name if we are in "teams" or "members" level
            if level == "teams" || level == "members" {
                let ws_id = selected_directory_workspace.read().clone();
                let ws_name = workspaces
                    .iter()
                    .find(|w| w.id == ws_id)
                    .map(|w| w.name.clone())
                    .unwrap_or_else(|| "Workspace".to_string());

                let mut directory_level = directory_level;
                let mut selected_directory_team = selected_directory_team;

                list.push(BreadcrumbItem {
                    label: ws_name,
                    onclick: Callback::new(move |_| {
                        directory_level.set("teams".to_string());
                        selected_directory_team.set(None);
                    }),
                });
            }

            // Push Team Name if we are in "members" level and have a selected team
            if level == "members"
                && let Some(ref team_id) = *selected_directory_team.read()
            {
                let team_name = teams
                    .iter()
                    .find(|t| &t.id == team_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| "Team".to_string());

                let mut directory_level = directory_level;

                list.push(BreadcrumbItem {
                    label: team_name,
                    onclick: Callback::new(move |_| {
                        directory_level.set("members".to_string());
                    }),
                });
            }
        }
        "scheduling" => {
            let date_str = selected_calendar_date.read().clone();
            list.push(BreadcrumbItem {
                label: date_str,
                onclick: Callback::new(|_| {}),
            });
        }
        _ => {}
    }

    list
}
