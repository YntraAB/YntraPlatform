---
name: i18n-localization
description: Guidelines for managing the multilingual Fluent localization system, modifying .ftl translation files, and invoking translation hooks in the Dioxus UI.
---

# Internationalization and Localization (i18n) Guide

This skill describes how to localized text in the Yntra UI layer. The system supports dynamic, local-first localization for five languages: English (`en`), Swedish (`sv`), Norwegian (`no`), Danish (`da`), and Finnish (`fi`).

---

## 1. Directory Structure

All translation keys are defined in Fluent translation files (`.ftl`) located inside the [locales directory](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/locales/):

- [en.ftl (English)](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/locales/en.ftl)
- [sv.ftl (Swedish)](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/locales/sv.ftl)
- [no.ftl (Norwegian)](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/locales/no.ftl)
- [da.ftl (Danish)](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/locales/da.ftl)
- [fi.ftl (Finnish)](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/locales/fi.ftl)

> [!IMPORTANT]
> When adding, renaming, or removing translation keys, you **MUST** modify all five files to ensure keys are synced across languages. If a key is missing in a language, it will automatically fall back to the English (`en`) translation.

---

## 2. Fluent Translation Syntax

Fluent uses a declarative syntax for localized labels. Placeholders and arguments are enclosed in brackets:

### Plain Keys
```ini
sidebar-dashboard = Dashboard
sidebar-scheduling = Scheduling
```

### Parameterized Keys (Variables)
Variables must be prefixed with `$` and wrapped in braces `{ $variable }`:
```ini
welcome-back-message = Welcome back, { $name }!
login-hw-error-card-unregistered = The card with ID { $id } is not registered in the system.
```

---

## 3. Implementing Translation in Rust Views

The translation engine is exposed via [`yntra-ui/src/locales.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/src/locales.rs).

### Fetching Locale State
Obtain the user's active locale from the global `AppState`:
```rust
let locale = state.auth_region.read(); // Yields "en", "sv", "no", "da", or "fi"
```

### Translation API

1. **Simple Translations**: Use `t(key, locale)`:
   ```rust
   use crate::locales::t;

   let label = t("sidebar-dashboard", &locale);
   ```

2. **Parameterized Translations**: Use `t_with_args(key, locale, &[("key", "value")])`:
   ```rust
   use crate::locales::t_with_args;

   let welcome = t_with_args(
       "welcome-back-message",
       &locale,
       &[("name", &username)]
   );
   ```

---

## 4. Best Practices

- **Semantic Key Naming**: Prefix keys based on the section they reside in (e.g. `auth-`, `settings-`, `messaging-`, `sidebar-`).
- **No Hardcoded UI Strings**: Do not place plain text strings (like `"Save"` or `"Cancel"`) directly in HTML elements. Always register them under a translation key.
- **Graceful Fallbacks**: The formatting logic handles missing keys by returning the key name or falling back to the `en` file. Ensure `en.ftl` is always fully up to date.
