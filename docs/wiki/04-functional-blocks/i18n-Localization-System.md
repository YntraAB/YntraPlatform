# Internationalization (i18n) & Fluent Localization

Yntra Platform incorporates a compile-time **Mozilla Fluent localization system** ([`yntra-ui/src/locales.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui/src/locales.rs)) supporting complete translation coverage across Nordic and English locales.

---

## 1. Supported Languages & FTL Bundles

| Locale Code | Language | FTL Resource File |
| :--- | :--- | :--- |
| `sv` | Swedish (Svenska) | `locales/sv.ftl` |
| `no` | Norwegian (Norsk) | `locales/no.ftl` |
| `da` | Danish (Dansk) | `locales/da.ftl` |
| `fi` | Finnish (Suomi) | `locales/fi.ftl` |
| `en` | English (Default Fallback) | `locales/en.ftl` |

---

## 2. Compile-Time Resource Inclusion & Performance

All `.ftl` translation files are bundled into binary memory at compile-time using `include_str!`:

```rust
const LOCALE_SV: &str = include_str!("../locales/sv.ftl");
const LOCALE_EN: &str = include_str!("../locales/en.ftl");
```

### Static Caching Mechanism
To eliminate redundant string parsing during UI re-renders, parsed translations are cached in a thread-local static lookup table (`STATIC_CACHE`):

* **WASM Engine**: Reads system language via `web_sys::window().navigator().language()`.
* **Native Desktop Engine**: Reads environment variables (`LANG`, `LC_ALL`).

---

## 3. UI Component Usage (`t!` Macro)

UI components invoke Fluent translations dynamically using parameter interpolation:

```rust
// Basic string translation
let label = t!("common-save");

// Parameterized string translation
let welcome_msg = t!("dashboard-welcome", name = user.name.as_str());
```
