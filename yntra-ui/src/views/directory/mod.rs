use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

mod members;
mod role_manager;
mod team_wizard;
mod teams;
mod template_manager;
mod workspaces;

pub use members::MembersList;
pub use role_manager::RoleManagerDialog;
pub use teams::TeamsList;
pub use template_manager::TemplateManagerDialog;
pub use workspaces::WorkspacesList;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WorkspaceRolePermissions {
    pub can_manage_schedule: bool,
    pub can_manage_notes: bool,
    pub can_approve_time_reports: bool,

    // School module
    #[serde(default)]
    pub can_manage_students: bool,
    #[serde(default)]
    pub can_manage_grades: bool,
    #[serde(default)]
    pub can_publish_report_cards: bool,
    #[serde(default)]
    pub can_access_health_records: bool,
    #[serde(default)]
    pub can_manage_billing: bool,
    #[serde(default)]
    pub can_manage_library: bool,

    // Care module
    #[serde(default)]
    pub can_manage_clients: bool,
    #[serde(default)]
    pub can_view_journals: bool,
    #[serde(default)]
    pub can_write_journals: bool,
    #[serde(default)]
    pub can_view_medications: bool,
    #[serde(default)]
    pub can_manage_medications: bool,

    // Moving company module
    #[serde(default)]
    pub can_manage_jobs: bool,
    #[serde(default)]
    pub can_manage_quotes: bool,
    #[serde(default)]
    pub can_complete_jobs: bool,

    // Core system module
    #[serde(default)]
    pub can_manage_workspace: bool,
    #[serde(default)]
    pub can_manage_users: bool,
    #[serde(default)]
    pub can_view_audit_logs: bool,
    #[serde(default)]
    pub can_submit_reports: bool,
    #[serde(default)]
    pub can_manage_reports: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct WorkspaceRole {
    pub id: String,
    pub name: String,
    pub permissions: WorkspaceRolePermissions,
}

// WizardClientForm was moved to team_wizard.rs

#[derive(Props, Clone)]
pub struct DirectoryViewProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub directory_level: Signal<String>,
    pub selected_directory_workspace: Signal<String>,
    pub selected_directory_team: Signal<Option<String>>,
    pub show_add_team_modal: Signal<bool>,
    pub show_invite_member_modal: Signal<bool>,
    pub show_client_manager_modal: Signal<bool>,
    pub new_team_name: Signal<String>,
    pub new_member_name: Signal<String>,
    pub new_member_email: Signal<String>,
    pub new_member_role: Signal<String>,
    pub new_client_first_name: Signal<String>,
    pub new_client_last_name: Signal<String>,
    pub new_client_personal_number: Signal<String>,
    pub new_client_care_level: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for DirectoryViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn DirectoryView(props: DirectoryViewProps) -> Element {
    let active_user = props.active_user;
    let workspace = props.workspace;

    // Local dialog visibility signals
    let show_template_manager_modal = use_signal(|| false);
    let show_role_manager_modal = use_signal(|| false);

    let show_add_team_modal = props.show_add_team_modal;
    let new_team_name = props.new_team_name;
    let db_trigger = props.db_trigger;

    // Local wizard signals were moved to team_wizard.rs

    let is_platform_admin = active_user.role == "platform_admin";
    let is_admin = active_user.role == "platform_admin" || active_user.role == "admin";

    let directory_level = props.directory_level;
    let _selected_directory_workspace = props.selected_directory_workspace;
    let selected_directory_team = props.selected_directory_team;

    let user_prefs: serde_json::Value =
        serde_json::from_str(&active_user.preferences).unwrap_or_default();
    let region = user_prefs
        .get("language")
        .and_then(|l| l.as_str())
        .unwrap_or("US")
        .to_string();

    let current_dir_level = directory_level.read().clone();
    let current_team = selected_directory_team.read().clone();
    let team_id_unwrap = current_team.clone().unwrap_or_else(|| "team-1".to_string());

    // Parse Custom Workspace Roles
    let settings_val: serde_json::Value =
        serde_json::from_str(&workspace.settings).unwrap_or_default();
    let custom_roles: Vec<WorkspaceRole> = settings_val
        .get("roles")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Step background colours were moved to team_wizard.rs

    let ws_id = workspace.id.clone();
    let modules_active: serde_json::Value =
        serde_json::from_str(&workspace.modules_active).unwrap_or_default();
    let is_school = modules_active
        .get("academics")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut active_tab = use_signal(|| "staff".to_string());

    rsx! {
        div { class: "flex flex-col h-full w-full bg-background box-border overflow-hidden",
            if is_school {
                div { class: "flex items-center gap-4 border-b border-border px-6 py-2 bg-muted/10 shrink-0",
                    button {
                        class: format!(
                            "text-xs font-semibold px-3 py-1.5 rounded-lg transition-colors {}",
                            if *active_tab.read() == "staff" { "bg-primary text-primary-foreground" } else { "text-muted-foreground hover:bg-muted" }
                        ),
                        onclick: move |_| active_tab.set("staff".to_string()),
                        "Staff & Teams"
                    }
                    button {
                        class: format!(
                            "text-xs font-semibold px-3 py-1.5 rounded-lg transition-colors {}",
                            if *active_tab.read() == "students" { "bg-primary text-primary-foreground" } else { "text-muted-foreground hover:bg-muted" }
                        ),
                        onclick: move |_| active_tab.set("students".to_string()),
                        "Students & Parents"
                    }
                }
            }

            // Sub-view Routing
            if is_school && *active_tab.read() == "students" {
                crate::views::school::StudentDirectoryView {
                    active_user_id: active_user.id.clone(),
                    workspace_id: workspace.id.clone(),
                    block_id: "students".to_string(),
                    db_trigger: db_trigger,
                    locale: region.clone(),
                }
            } else if current_dir_level == "workspaces" {
                WorkspacesList {
                    is_platform_admin,
                }
            } else if current_dir_level == "teams" {
                TeamsList {
                    is_admin,
                    is_platform_admin,
                    show_role_manager_modal,
                    show_template_manager_modal,
                }
            } else {
                MembersList {
                    active_user: active_user.clone(),
                    team_id_unwrap: team_id_unwrap.clone(),
                    custom_roles: custom_roles.clone(),
                }
            }
        }

        // Modals
        RoleManagerDialog {
            show_role_manager_modal,
            custom_roles,
            settings_val,
        }

        TemplateManagerDialog {
            show_template_manager_modal,
            region: region.clone(),
        }

        // Multi-Step Team Registration Wizard Dialog
        team_wizard::TeamRegistrationWizard {
            show_add_team_modal,
            new_team_name,
            active_user: active_user.clone(),
            workspace_id: ws_id,
            db_trigger,
            region: region.clone(),
        }
    }
}
