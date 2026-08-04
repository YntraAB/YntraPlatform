---
title: "Dynamic Entity Schemas & Custom Forms"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Yntra allows non-technical administrators to define custom operational data models using JSON schema specifications.

---

## 1. Schema JSON Specification

```json
[
  {
    "key": "client_name",
    "label": "Customer Name",
    "type": "text",
    "required": true
  },
  {
    "key": "priority_level",
    "label": "Priority Level",
    "type": "select",
    "options": ["Low", "Medium", "High", "Urgent"],
    "required": true
  }
]
```

---

## 2. Cross-Platform Native Form Builders

- **Web & Desktop (Dioxus)**: Rendered via [`dynamic_form.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-ui/src/components/dynamic_form.rs).
- **Android (Jetpack Compose)**: Rendered via [`DynamicBlockView.kt`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-android/app/src/main/java/com/yntra/app/views/DynamicBlockView.kt).
- **iOS (SwiftUI)**: Rendered via [`DynamicBlockView.swift`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-ios/YntraIOS/Views/DynamicBlockView.swift).