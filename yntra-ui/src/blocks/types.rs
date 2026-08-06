#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockDomainCategory {
    CoreWorkOS,
    EnterpriseHealthcare,
    EnterpriseAcademics,
    FieldAndLogistics,
    RegionalRegulatory,
}

impl BlockDomainCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockDomainCategory::CoreWorkOS => "core_work_os",
            BlockDomainCategory::EnterpriseHealthcare => "enterprise_healthcare",
            BlockDomainCategory::EnterpriseAcademics => "enterprise_academics",
            BlockDomainCategory::FieldAndLogistics => "field_and_logistics",
            BlockDomainCategory::RegionalRegulatory => "regional_regulatory",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            BlockDomainCategory::CoreWorkOS => "Core Work OS",
            BlockDomainCategory::EnterpriseHealthcare => "Enterprise Healthcare (HL7/FHIR)",
            BlockDomainCategory::EnterpriseAcademics => "Academic Suite (LTI/Canvas)",
            BlockDomainCategory::FieldAndLogistics => "Field Ops & Logistics",
            BlockDomainCategory::RegionalRegulatory => "Regional Compliance & Tax",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleTier {
    CorePlatform,
    EnterpriseVertical,
    RegionalExtension,
}

impl ModuleTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModuleTier::CorePlatform => "core_platform",
            ModuleTier::EnterpriseVertical => "enterprise_vertical",
            ModuleTier::RegionalExtension => "regional_extension",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnterpriseStandardsSpec {
    pub compliance_standards: &'static [&'static str],
    pub supported_protocols: &'static [&'static str],
    pub enterprise_connectors: &'static [&'static str],
    pub jurisdiction: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub domain_categories: &'static [BlockDomainCategory],
    pub tier: ModuleTier,
    pub standards: EnterpriseStandardsSpec,
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
    pub domain_category: BlockDomainCategory,
    pub included_block_ids: &'static [&'static str],
    pub default_role_profile: &'static str,
}
