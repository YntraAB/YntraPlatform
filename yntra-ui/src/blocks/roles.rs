use super::types::RoleProfileDefinition;

pub static ROLE_PROFILES: &[RoleProfileDefinition] = &[
    RoleProfileDefinition {
        role_id: "field_worker",
        display_name: "Field Worker (Task-First)",
        primary_navigation_ids: &["dashboard", "jobs", "messaging"],
        is_streamlined_field_mode: true,
    },
    RoleProfileDefinition {
        role_id: "supervisor",
        display_name: "Operations Supervisor",
        primary_navigation_ids: &["dashboard", "dispatch", "live_map", "scheduling", "messaging"],
        is_streamlined_field_mode: false,
    },
    RoleProfileDefinition {
        role_id: "platform_admin",
        display_name: "Executive & System Admin",
        primary_navigation_ids: &["dashboard", "admin_panel", "finance", "ecosystem_integrations"],
        is_streamlined_field_mode: false,
    },
];
