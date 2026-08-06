pub mod registry;
pub mod roles;
pub mod suites;
pub mod types;

pub use registry::BLOCK_REGISTRY;
pub use roles::ROLE_PROFILES;
pub use suites::OPERATIONAL_SUITES;
pub use types::{
    BlockDefinition, BlockDomainCategory, BlockNavChild, BlockNavItem, EnterpriseStandardsSpec,
    ModuleTier, OperationalSuiteDefinition, RoleProfileDefinition,
};

pub fn get_block_by_id(id: &str) -> Option<&'static BlockDefinition> {
    BLOCK_REGISTRY.iter().find(|b| b.id == id)
}

pub fn get_blocks_by_domain(domain: BlockDomainCategory) -> Vec<&'static BlockDefinition> {
    BLOCK_REGISTRY
        .iter()
        .filter(|b| b.domain_categories.contains(&domain))
        .collect()
}

pub fn get_regional_extensions() -> Vec<&'static BlockDefinition> {
    BLOCK_REGISTRY
        .iter()
        .filter(|b| b.tier == ModuleTier::RegionalExtension)
        .collect()
}
