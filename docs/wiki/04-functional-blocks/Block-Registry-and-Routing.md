# Block Registry & View Routing Guide

Yntra features a **Modular Operational OS Architecture**. Workspace administrators enable functional tools (Messaging, Time Clock, Care Assistance, Vehicle Inspection, School Operations) by toggling block keys in the workspace definition.

---

## 1. Registering a Functional Block with Enterprise Standards

Functional blocks are defined in the global static array [`BLOCK_REGISTRY` in `yntra-ui/src/blocks/registry.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-ui/src/blocks/registry.rs). Each block specifies its **Domain Category** (`CoreWorkOS`, `EnterpriseHealthcare`, `EnterpriseAcademics`, `FieldAndLogistics`, `RegionalRegulatory`), **Module Tier**, and **Enterprise Standards & Regulatory Compliance Metadata** (e.g. `HL7-FHIR-R4`, `IMS-LTI-1.3`, `DICOM-Web`, `FERPA`, `HIPAA`, `Skatteverket-v2.1`):

```rust
// yntra-ui/src/blocks/registry.rs
pub static BLOCK_REGISTRY: &[BlockDefinition] = &[
    BlockDefinition {
        id: "health_clinic",
        name: "Health Clinic EMR",
        domain_category: BlockDomainCategory::EnterpriseHealthcare,
        tier: ModuleTier::EnterpriseVertical,
        standards: EnterpriseStandardsSpec {
            compliance_standards: &["HL7-FHIR-R4", "DICOM-Web", "HIPAA-Security-Rule", "21-CFR-Part-11"],
            supported_protocols: &["REST/FHIR-JSON", "DICOM-STOW-RS", "HL7-v2-MLLP"],
            enterprise_connectors: &["Epic Systems EMR", "Cerner Millennium"],
            jurisdiction: "US-HIPAA / EU-MDR",
        },
        navigation: &[BlockNavItem {
            id: "health_clinic",
            label_key: "settings-blocks-health-clinic-name",
            path: "health_clinic",
            icon: "heart",
            allowed_roles: None,
            section: "main",
            children: None,
            badge_key: None,
        }],
    },
    // ...
];
```

:::tip
To add a new operational block, append a new `BlockDefinition` entry to `BLOCK_REGISTRY` in `yntra-ui/src/blocks/registry.rs` with domain taxonomy and standards metadata.
:::

---

## 2. Dynamic Navigation Filtering

Navbars on Web/Desktop (Dioxus) and Mobile (SwiftUI/Compose) filter visible tabs dynamically based on `workspace.modules_active` JSON definitions.
