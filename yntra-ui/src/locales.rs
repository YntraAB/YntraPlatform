use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

const LOCALE_SV: &str = include_str!("../locales/sv.ftl");
const LOCALE_NO: &str = include_str!("../locales/no.ftl");
const LOCALE_DA: &str = include_str!("../locales/da.ftl");
const LOCALE_FI: &str = include_str!("../locales/fi.ftl");
const LOCALE_EN: &str = include_str!("../locales/en.ftl");

type BundleMap = HashMap<String, fluent_bundle::FluentBundle<fluent_bundle::FluentResource>>;

thread_local! {
    static TRANSLATION_BUNDLES: RefCell<BundleMap> = RefCell::new(HashMap::new());
}

static STATIC_CACHE: OnceLock<RwLock<HashMap<(String, String), String>>> = OnceLock::new();
static SYSTEM_LOCALE: OnceLock<String> = OnceLock::new();

pub fn get_system_locale() -> String {
    SYSTEM_LOCALE
        .get_or_init(|| {
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
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Ok(lang) = std::env::var("LANG").or_else(|_| std::env::var("LC_ALL")) {
                    let lang = lang.to_lowercase();
                    if lang.starts_with("sv") {
                        return "sv".to_string();
                    } else if lang.starts_with("nb") || lang.starts_with("nn") || lang.starts_with("no") {
                        return "no".to_string();
                    } else if lang.starts_with("da") {
                        return "da".to_string();
                    } else if lang.starts_with("fi") {
                        return "fi".to_string();
                    }
                }

                #[cfg(target_os = "windows")]
                {
                    unsafe extern "system" {
                        fn GetUserDefaultLocaleName(lpLocaleName: *mut u16, cchLocaleName: i32) -> i32;
                    }
                    let mut buf = [0u16; 85];
                    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
                    if len > 0 {
                        if let Ok(locale_str) = String::from_utf16(&buf[.. (len as usize - 1)]) {
                            let lang = locale_str.to_lowercase();
                            if lang.starts_with("sv") {
                                return "sv".to_string();
                            } else if lang.starts_with("nb") || lang.starts_with("nn") || lang.starts_with("no") {
                                return "no".to_string();
                            } else if lang.starts_with("da") {
                                return "da".to_string();
                            } else if lang.starts_with("fi") {
                                return "fi".to_string();
                            }
                        }
                    }
                }
            }
            "en".to_string()
        })
        .clone()
}

pub fn t(key: &str, locale: &str) -> String {
    let cache = STATIC_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    let cache_key = (key.to_string(), locale.to_string());

    if let Ok(guard) = cache.read() {
        if let Some(val) = guard.get(&cache_key) {
            return val.clone();
        }
    }

    let val = t_with_args(key, locale, &[]);

    if let Ok(mut guard) = cache.write() {
        guard.insert(cache_key, val.clone());
    }

    val
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
            && let Some(pattern) = msg.value()
        {
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
            && let Some(pattern) = msg.value()
        {
            let mut fluent_args = fluent_bundle::FluentArgs::new();
            for &(k, v) in args {
                fluent_args.set(k, v);
            }
            let mut errors = vec![];
            let formatted = en_bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
            return formatted.to_string();
        }

        key.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locales_memoization_cache() {
        let sv_val1 = t("nav_dashboard", "sv");
        let sv_val2 = t("nav_dashboard", "sv");
        assert_eq!(sv_val1, sv_val2);

        let en_val = t("nav_dashboard", "en");
        assert!(!en_val.is_empty());
    }
}
