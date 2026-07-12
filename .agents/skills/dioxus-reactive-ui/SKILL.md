---
name: dioxus-reactive-ui
description: Guidelines and patterns for Dioxus 0.7+ fine-grained signals, resource caching, desktop cross-platform compatibility, visual aesthetics styling, and architectural boundaries.
---

# Dioxus 0.7+ Reactive UI Guide

This skill defines the development guidelines, patterns, and style rules for building views and components in the Yntra UI layer (`yntra-ui`).

---

## 1. Fine-Grained Reactivity (Signals)

Dioxus 0.7 uses fine-grained signal reactivity. Legacy hook APIs like `use_state` or `use_ref` are deprecated and must not be used.

### Signals (`use_signal`)
* Always use `use_signal` to define local component state.
* To read a signal's value, use `.read()` (which creates a subscription dependency).
* To mutate a signal's value, use `.set(...)` or `.write()`.
* See [examples/counter.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/dioxus-reactive-ui/examples/counter.rs) for a complete signal reactivity implementation.

### Memoized Signals (`use_memo`)
* Use `use_memo` to compute derived state. This prevents recalculations on every render.
  ```rust
  let count = use_signal(|| 0);
  let double_count = use_memo(move || count.read() * 2);
  ```

### Side Effects (`use_effect`)
* Use `use_effect` to perform side effects (e.g., subscribing to database updates or performing actions when signals change).
  ```rust
  use_effect(move || {
      let current_val = count.read();
      println!("Count changed to: {}", current_val);
  });
  ```

---

## 2. Asynchronous Resources (`use_resource`)

* For fetching data asynchronously, always use `use_resource`.
* Do not block the UI thread. Let Dioxus resolve the future dynamically.
* To force-refresh the resource, call `.restart()`.
* See [examples/workspace_viewer.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/dioxus-reactive-ui/examples/workspace_viewer.rs) for a complete resource data fetching implementation.

---

## 3. Desktop & Web Cross-Platform Compatibility

All frontend dependencies must compile on both WASM (Web) and Wry/GPUI (Desktop). 

* **No direct std::fs / native paths**: Never import `std::fs`, `std::env`, or similar native filesystem libraries in UI views.
* **Feature flags / Conditional imports**: If target-specific code is necessary, isolate it using `#[cfg(target_arch = "wasm32")]` or `#[cfg(not(target_arch = "wasm32"))]`.
* **Platform-neutral wrappers**: Keep custom window controls or native desktop integrations separated into `utils/` or the logic core.

---

## 4. Aesthetic & Styling Rules

To ensure a premium, modern design, follow these guidelines:
* **Typography**: Utilize Outfit or Inter styling. Do not use default serif fonts.
* **Tailwind + Custom Animations**: Combine Tailwind styling with subtle micro-animations (e.g., `transition-all duration-200 hover:scale-102 hover:shadow-lg active:scale-98`).
* **HSL Color System**: Favor harmonious, CSS-defined custom HSL variables over generic utility colors (e.g., use `var(--primary)` instead of hardcoded `bg-blue-500` to support smooth dark/light mode transitions).
* **Loading States**: Always provide skeleton loader designs (`animate-pulse`) instead of empty pages during asynchronous transitions.

---

## 5. Architectural Integrity

* **Never bypass the core database**: Views must never contact REST/GraphQL APIs, Supabase directly, or read raw filesystems.
* **Observe, Don't Poll**: Subscribe to core changes using `DatabaseObserver` callbacks, which write to Dioxus signals.
