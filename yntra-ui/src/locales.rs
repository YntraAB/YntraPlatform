const LOCALE_SV: &str = include_str!("../locales/sv.ftl");
const LOCALE_NO: &str = include_str!("../locales/no.ftl");
const LOCALE_DA: &str = include_str!("../locales/da.ftl");
const LOCALE_FI: &str = include_str!("../locales/fi.ftl");
const LOCALE_EN: &str = include_str!("../locales/en.ftl");

thread_local! {
    static TRANSLATION_BUNDLES: std::cell::RefCell<std::collections::HashMap<String, fluent_bundle::FluentBundle<fluent_bundle::FluentResource>>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

pub fn get_system_locale() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(lang) = window.navigator().language() {
                let lang = lang.to_lowercase();
                if lang.starts_with("sv") {
                    return "sv".to_string();
                } else if lang.starts_with("nb") || lang.starts_with("nn") || lang.starts_with("no")
                {
                    return "no".to_string();
                } else if lang.starts_with("da") {
                    return "da".to_string();
                } else if lang.starts_with("fi") {
                    return "fi".to_string();
                }
            }
        }
    }
    "en".to_string()
}

pub fn t(key: &str, locale: &str) -> String {
    t_with_args(key, locale, &[])
}

pub fn t_with_args(key: &str, locale: &str, args: &[(&str, &str)]) -> String {
    TRANSLATION_BUNDLES.with(|bundles| {
        let mut bundles_borrow = bundles.borrow_mut();
        if bundles_borrow.is_empty() {
            for (lang, source) in [
                ("sv", LOCALE_SV),
                ("no", LOCALE_NO),
                ("da", LOCALE_DA),
                ("fi", LOCALE_FI),
                ("en", LOCALE_EN),
            ] {
                let res = fluent_bundle::FluentResource::try_new(source.to_string())
                    .expect("Failed to parse an FTL resource.");

                let lid: unic_langid::LanguageIdentifier = lang.to_lowercase().parse().unwrap();
                let mut bundle = fluent_bundle::FluentBundle::new(vec![lid]);
                bundle
                    .add_resource(res)
                    .expect("Failed to add resource to bundle.");
                bundles_borrow.insert(lang.to_string(), bundle);
            }
        }

        // Find bundle for target locale, fallback to en
        let bundle = bundles_borrow
            .get(locale)
            .unwrap_or_else(|| bundles_borrow.get("en").expect("en bundle must exist"));

        // Try looking up the message in target locale
        if let Some(msg) = bundle.get_message(key)
            && let Some(pattern) = msg.value() {
                let mut fluent_args = fluent_bundle::FluentArgs::new();
                for &(k, v) in args {
                    fluent_args.set(k, v);
                }
                let mut errors = vec![];
                let formatted = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
                return formatted.to_string();
            }

        // Key-by-key fallback chain: if missing from target locale, try "en"
        if locale != "en"
            && let Some(en_bundle) = bundles_borrow.get("en")
                && let Some(msg) = en_bundle.get_message(key)
                    && let Some(pattern) = msg.value() {
                        let mut fluent_args = fluent_bundle::FluentArgs::new();
                        for &(k, v) in args {
                            fluent_args.set(k, v);
                        }
                        let mut errors = vec![];
                        let formatted =
                            en_bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
                        return formatted.to_string();
                    }

        key.to_string()
    })
}
