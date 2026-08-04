# Dynamic Entity Schemas & Custom Forms

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

- **Web & Desktop (Dioxus)**: Rendered via [`dynamic_form.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/src/components/dynamic_form.rs).
- **Android (Jetpack Compose)**: Rendered via [`DynamicBlockView.kt`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-android/app/src/main/java/com/yntra/app/views/DynamicBlockView.kt).
- **iOS (SwiftUI)**: Rendered via [`DynamicBlockView.swift`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ios/YntraIOS/Views/DynamicBlockView.swift).
