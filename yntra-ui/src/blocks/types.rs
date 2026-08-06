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

#[derive(Clone, Debug, PartialEq)]
pub struct RoleProfileDefinition {
    pub role_id: &'static str,
    pub display_name: &'static str,
    pub primary_navigation_ids: &'static [&'static str],
    pub is_streamlined_field_mode: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationalSuiteDefinition {
    pub suite_id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub included_block_ids: &'static [&'static str],
    pub default_role_profile: &'static str,
}
