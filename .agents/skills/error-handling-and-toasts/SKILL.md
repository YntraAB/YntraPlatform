---
name: error-handling-and-toasts
description: Guidelines for handling database errors, executing UI mutations using ActionRunner, and showing user-facing toast alerts.
---

# Error Handling and Toasts Guidelines

This skill guides how asynchronous actions, database writes, and error states are handled and communicated to the user in the Yntra UI layer.

---

## 1. Action Execution via `ActionRunner`

For non-blocking database writes, updates, and other asynchronous mutations triggered by user events (like button clicks), use the `ActionRunner` utility. This utility executes the future in the background and automatically catches any errors, converting them into user-friendly notifications.

### Implementation Blueprint
Obtain the runner in a view or component using `use_action_runner()`, then dispatch the asynchronous mutation:

```rust
use crate::utils::use_action_runner;
use yntra_core::save_inventory_item;

#[component]
pub fn SaveButton(item: InventoryItem) -> Element {
    let runner = use_action_runner();

    rsx! {
        button {
            onclick: move |_| {
                let item_clone = item.clone();
                // Dispatch action asynchronously. Errors will be caught and toasted.
                runner.run(async move {
                    save_inventory_item(item_clone).await
                });
            },
            "Save Changes"
        }
    }
}
```

---

## 2. Managing Local Component State via `use_action`

When a component needs to track loading/error states locally (e.g. to display a loading spinner or disable a submit button during execution), use the `use_action` hook:

```rust
use crate::utils::use_action;

let submit_action = use_action(move || {
    let credentials = credentials_signal.read().clone();
    async move {
        login_user(credentials).await
    }
});

rsx! {
    button {
        disabled: submit_action.is_loading(),
        onclick: move |_| submit_action.run(),
        if submit_action.is_loading() {
            "Logging in..."
        } else {
            "Submit"
        }
    }
}
```

---

## 3. Error Mapping System

All low-level database, networking, and cryptographic errors (`yntra_core::YntraError`) must be mapped to clear, user-facing error details. This logic is handled centrally in [`yntra-ui/src/utils/errors.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/src/utils/errors.rs) via `map_error`.

Supported mappings include:
- **`AuthError`** $\rightarrow$ access denied alert (missing permissions).
- **`DbError`** $\rightarrow$ failed write alert (local database corruption/issues).
- **`NetworkError`** $\rightarrow$ network connection failed alerts.
- **`CryptoError`** $\rightarrow$ security validation failures.
- **`ConstraintError`** $\rightarrow$ database constraint violation notifications.

### Best Practices
- **Do not expose raw error messages** (like SQLite transaction logs) directly to users. Always route them through `map_error` or wrap them in standard user-friendly text.
- **Always block double-submissions** by checking loading states or disabling input buttons during active async tasks.
