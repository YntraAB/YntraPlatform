#[derive(Clone, Debug, PartialEq)]
pub struct BlockDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub navigation: &'static [BlockNavItem],
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockNavItem {
    pub id: &'static str,
    pub label_key: &'static str,
    pub path: &'static str,
    pub icon: &'static str,
    pub allowed_roles: Option<&'static [&'static str]>,
    pub section: &'static str,
    pub children: Option<&'static [BlockNavChild]>,
    pub badge_key: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockNavChild {
    pub id: &'static str,
    pub label_key: &'static str,
    pub path: &'static str,
    pub icon: &'static str,
    pub required_block_id: Option<&'static str>,
}

pub static BLOCK_REGISTRY: &[BlockDefinition] = &[
    BlockDefinition {
        id: "dashboard",
        name: "Dashboard",
        navigation: &[
            BlockNavItem {
                id: "dashboard",
                label_key: "section-dashboard",
                path: "dashboard",
                icon: "dashboard",
                allowed_roles: None,
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "messaging",
        name: "Messaging",
        navigation: &[
            BlockNavItem {
                id: "messaging",
                label_key: "section-messaging",
                path: "messaging",
                icon: "messaging",
                allowed_roles: None,
                section: "main",
                children: None,
                badge_key: Some("unread_messages"),
            }
        ]
    },
    BlockDefinition {
        id: "scheduling",
        name: "Scheduling",
        navigation: &[
            BlockNavItem {
                id: "time_group",
                label_key: "sidebar-time-management",
                path: "scheduling",
                icon: "clock",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant"]),
                section: "main",
                badge_key: None,
                children: Some(&[
                    BlockNavChild {
                        id: "scheduling",
                        label_key: "section-scheduling",
                        path: "scheduling",
                        icon: "scheduling",
                        required_block_id: None,
                    },
                    BlockNavChild {
                        id: "time",
                        label_key: "section-time",
                        path: "time",
                        icon: "time",
                        required_block_id: Some("time"),
                    }
                ]),
            }
        ]
    },
    BlockDefinition {
        id: "notes",
        name: "Notes",
        navigation: &[
            BlockNavItem {
                id: "notes",
                label_key: "section-notes",
                path: "notes",
                icon: "notes",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant"]),
                section: "main",
                children: None,
                badge_key: Some("unread_notes"),
            }
        ]
    },
    BlockDefinition {
        id: "journals",
        name: "Care Journals",
        navigation: &[
            BlockNavItem {
                id: "journals",
                label_key: "section-journals",
                path: "journals",
                icon: "notes",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "medications",
        name: "Medications",
        navigation: &[
            BlockNavItem {
                id: "medications",
                label_key: "section-medications",
                path: "medications",
                icon: "pill",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "client_portal",
        name: "Client Portal",
        navigation: &[
            BlockNavItem {
                id: "client_portal",
                label_key: "section-assistance",
                path: "client_portal",
                icon: "heart",
                allowed_roles: Some(&["client"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "directory",
        name: "Directory",
        navigation: &[
            BlockNavItem {
                id: "directory",
                label_key: "section-directory",
                path: "directory",
                icon: "directory",
                allowed_roles: None,
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "reporting",
        name: "Reporting",
        navigation: &[
            BlockNavItem {
                id: "reporting",
                label_key: "section-reporting",
                path: "reporting",
                icon: "reporting",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "jobs",
        name: "Jobs",
        navigation: &[
            BlockNavItem {
                id: "jobs",
                label_key: "jobs-nav",
                path: "jobs",
                icon: "jobs",
                allowed_roles: None,
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "todos",
        name: "Todos",
        navigation: &[
            BlockNavItem {
                id: "todos",
                label_key: "todos-nav",
                path: "todos",
                icon: "check-square",
                allowed_roles: None,
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "academics",
        name: "Academics",
        navigation: &[
            BlockNavItem {
                id: "academics",
                label_key: "section-academics",
                path: "academics",
                icon: "graduation-cap",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant", "parent"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "attendance",
        name: "Attendance",
        navigation: &[
            BlockNavItem {
                id: "attendance",
                label_key: "section-attendance",
                path: "attendance",
                icon: "check-circle",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant", "parent"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "finance",
        name: "Finance",
        navigation: &[
            BlockNavItem {
                id: "finance",
                label_key: "section-finance",
                path: "finance",
                icon: "credit-card",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant", "parent"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    },
    BlockDefinition {
        id: "library",
        name: "Library",
        navigation: &[
            BlockNavItem {
                id: "library",
                label_key: "section-library",
                path: "library",
                icon: "book-open",
                allowed_roles: Some(&["platform_admin", "admin", "user", "assistant", "parent"]),
                section: "main",
                children: None,
                badge_key: None,
            }
        ]
    }
];

