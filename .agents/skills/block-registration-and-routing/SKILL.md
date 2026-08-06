---
name: block-registration-and-routing
description: Guidelines for registering new functional blocks in the UI, adding them to the BLOCK_REGISTRY, configuring navigation items/badges, and setting up routing.
---

# UI Block Registration and Routing Guidelines

This skill guides the implementation of new functional blocks or services in the Yntra UI client. It defines the structure, registry location, and navigation rules required to expose a feature through the Dioxus client.

---

## 1. Declarative Block Definitions

All UI blocks must be registered statically within [`yntra-ui/src/blocks.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/src/blocks.rs). A block consists of a unique identifier, a display name, and navigation layout settings.

### Schema Specifications

- **`BlockDefinition`**:
  * `id`: Unique string identifying the functional block (e.g. `"messaging"`).
  * `name`: Human-readable name used internally/fallback.
  * `domain_category`: `BlockDomainCategory` (`CoreWorkOS`, `EnterpriseHealthcare`, `EnterpriseAcademics`, `FieldAndLogistics`, `RegionalRegulatory`).
  * `tier`: `ModuleTier` (`CorePlatform`, `EnterpriseVertical`, `RegionalExtension`).
  * `standards`: `EnterpriseStandardsSpec` (`compliance_standards`, `supported_protocols`, `enterprise_connectors`, `jurisdiction`).
  * `navigation`: Static list of main navigation items (`BlockNavItem`).

- **`BlockNavItem`**:
  * `id`: Sub-element ID.
  * `label_key`: Localization key defined in Fluent translation files (e.g. `"section-messaging"`).
  * `path`: View routing path.
  * `icon`: Display icon name.
  * `allowed_roles`: List of roles permitted to view/interact with this navigation block. Use `None` for public/any roles.
  * `section`: Sidebar or content grouping section (typically `"main"`).
  * `children`: Nested sub-items (`BlockNavChild`), or `None` if flat.
  * `badge_key`: Optional reactive badge identifier (e.g., `"unread_messages"`).

- **`BlockNavChild`**:
  * `id`: Child item ID.
  * `label_key`: Localization key.
  * `path`: View routing path.
  * `icon`: Display icon name.
  * `required_block_id`: Optional parent or auxiliary block requirement to unlock.

---

## 2. Implementation Blueprint

### Step A: Register the Block in [`yntra-ui/src/blocks/registry.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/src/blocks/registry.rs)

Append a new definition to the `BLOCK_REGISTRY` static array:

```rust
BlockDefinition {
    id: "inventory",
    name: "Inventory & Supply Chain",
    domain_category: BlockDomainCategory::FieldAndLogistics,
    tier: ModuleTier::CorePlatform,
    standards: EnterpriseStandardsSpec {
        compliance_standards: &["GS1-128", "ISO-28000"],
        supported_protocols: &["REST", "RFID-EPCIS"],
        enterprise_connectors: &["SAP S/4HANA", "Oracle SCM"],
        jurisdiction: "Global",
    },
    navigation: &[BlockNavItem {
        id: "inventory",
        label_key: "section-inventory",
        path: "inventory",
        icon: "archive",
        allowed_roles: Some(&["platform_admin", "admin"]),
        section: "main",
        badge_key: None,
        children: None,
    }],
}
```

### Step B: Map the View in [`main.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/src/main.rs)

The frontend routes components dynamically based on the active section signal. Map the registry path to a UI View inside the `App` component's layout block:

```rust
// In yntra-ui/src/main.rs inside the active section router match:
match active_section.read().as_str() {
    "dashboard" => rsx! { views::Dashboard {} },
    "messaging" => rsx! { views::Messaging {} },
    "inventory" => rsx! { views::Inventory {} }, // Register the new view component
    _ => rsx! { views::NotFound {} }
}
```

### Step C: Define the View Component
Create the view file in `yntra-ui/src/views/inventory.rs` and expose it in `yntra-ui/src/views/mod.rs`:

```rust
use dioxus::prelude::*;

#[component]
pub fn Inventory() -> Element {
    rsx! {
        div { class: "p-6 space-y-4",
            h1 { class: "text-2xl font-bold", "Inventory Management" }
            // Custom business logic and reactive database components
        }
    }
}
```

---

## 3. Best Practices

1. **Role Protection**: Always specify `allowed_roles` on the navigation item to restrict views to authorized user tiers (e.g., `client` vs `admin`).
2. **Localization Integration**: Always pair the navigation registration with a localization label key. Create keys in all translation files (like `en.ftl`, `sv.ftl`) mapping the key to the translated name.
3. **Module Isolation**: Keep view components modular within the `yntra-ui/src/views` folder.
