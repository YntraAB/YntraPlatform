# Block Registry & View Routing Guide

Yntra features a **Modular Operational OS Architecture**. Workspace administrators enable functional tools (Messaging, Time Clock, Care Assistance, Vehicle Inspection, School Operations) by toggling block keys in the workspace definition.

---

## 1. Registering a Functional Block

Functional blocks are defined in the global static array [`BLOCK_REGISTRY` in `yntra-ui/src/blocks.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-ui/src/blocks.rs#L29-L396):

```rust
// yntra-ui/src/blocks.rs:L29-L57
pub static BLOCK_REGISTRY: &[BlockDefinition] = &[
    BlockDefinition {
        id: "messaging",
        name: "Messaging",
        navigation: &[BlockNavItem {
            id: "messaging",
            label_key: "section-messaging",
            path: "messaging",
            icon: "messaging",
            allowed_roles: None,
            section: "main",
            children: None,
            badge_key: Some("unread_messages"),
        }],
    },
    // ...
];
```

:::tip
To add a new operational block, append a new [`BlockDefinition`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-ui/src/blocks.rs#L1-L6) entry to `BLOCK_REGISTRY` in `yntra-ui/src/blocks.rs#L29`.
:::

---

## 2. Dynamic Navigation Filtering

Navbars on Web/Desktop (Dioxus) and Mobile (SwiftUI/Compose) filter visible tabs dynamically based on `workspace.modules_active` JSON definitions.
