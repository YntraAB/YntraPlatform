use super::types::OperationalSuiteDefinition;

pub static OPERATIONAL_SUITES: &[OperationalSuiteDefinition] = &[
    OperationalSuiteDefinition {
        suite_id: "field_ops_suite",
        title: "Yntra Field Ops & Service OS",
        description: "Turnkey operational bundle combining Dispatch, Work Orders, Time Tracking, and Automated Job Costing.",
        included_block_ids: &["dashboard", "jobs", "dispatch", "time", "messaging"],
        default_role_profile: "field_worker",
    },
    OperationalSuiteDefinition {
        suite_id: "healthcare_suite",
        title: "Yntra Healthcare & Care Services OS",
        description: "Turnkey bundle combining Care Journals, Active Medication logs, Vitals, and Emergency Messaging alerts.",
        included_block_ids: &["dashboard", "journals", "medications", "client_portal", "messaging"],
        default_role_profile: "field_worker",
    },
    OperationalSuiteDefinition {
        suite_id: "logistics_suite",
        title: "Yntra Transport & Logistics OS",
        description: "Turnkey dispatch, fleet management, live map tracking, and crew communication.",
        included_block_ids: &["dashboard", "dispatch", "live_map", "fleet", "messaging"],
        default_role_profile: "supervisor",
    },
];
