use super::types::{BlockDomainCategory, OperationalSuiteDefinition};

pub static OPERATIONAL_SUITES: &[OperationalSuiteDefinition] = &[
    OperationalSuiteDefinition {
        suite_id: "field_ops_suite",
        title: "Yntra Field Ops & Service OS",
        description: "Turnkey operational bundle combining Dispatch, Work Orders, Time Tracking, and Automated Job Costing.",
        domain_category: BlockDomainCategory::FieldAndLogistics,
        included_block_ids: &["dashboard", "jobs", "dispatch", "time", "messaging"],
        default_role_profile: "field_worker",
    },
    OperationalSuiteDefinition {
        suite_id: "healthcare_suite",
        title: "Yntra Enterprise Healthcare OS (HL7/FHIR)",
        description: "Turnkey enterprise healthcare suite combining Care Journals, Active Medication logs, Health Clinic EMR, and Emergency Messaging alerts compliant with HL7 FHIR R4 and HIPAA.",
        domain_category: BlockDomainCategory::EnterpriseHealthcare,
        included_block_ids: &["dashboard", "health_clinic", "journals", "medications", "client_portal", "messaging"],
        default_role_profile: "field_worker",
    },
    OperationalSuiteDefinition {
        suite_id: "academic_suite",
        title: "Yntra Higher Ed & Academic OS (LTI 1.3/Canvas)",
        description: "Enterprise academic suite combining Academics, Student Attendance, Course Library, and Report Cards integrated with LTI 1.3 Advantage and FERPA audit logs.",
        domain_category: BlockDomainCategory::EnterpriseAcademics,
        included_block_ids: &["dashboard", "academics", "attendance", "library", "report_cards", "messaging"],
        default_role_profile: "supervisor",
    },
    OperationalSuiteDefinition {
        suite_id: "logistics_suite",
        title: "Yntra Transport & Logistics OS",
        description: "Turnkey dispatch, fleet management, live map tracking, and crew communication with GTFS telematics.",
        domain_category: BlockDomainCategory::FieldAndLogistics,
        included_block_ids: &["dashboard", "dispatch", "live_map", "fleet", "messaging"],
        default_role_profile: "supervisor",
    },
    OperationalSuiteDefinition {
        suite_id: "regional_compliance_suite",
        title: "Yntra Regional Compliance & Tax OS (SE-RUT)",
        description: "Jurisdiction-specific extensions bundle containing Swedish Skatteverket RUT/ROT tax deduction exports and Peppol e-Invoicing.",
        domain_category: BlockDomainCategory::RegionalRegulatory,
        included_block_ids: &["dashboard", "rut_exports", "finance", "admin_panel"],
        default_role_profile: "platform_admin",
    },
];
