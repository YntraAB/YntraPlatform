use super::super::ChecklistItem;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{JobTicket, MoveInventoryItem, MoveQuote};

#[derive(Props, Clone, PartialEq)]
pub struct JobDetailsProps {
    pub job: JobTicket,
    pub active_user_id: String,
    pub region: String,
    pub checklist_state: Signal<Vec<ChecklistItem>>,
    pub completion_report_state: Signal<String>,
    pub inventories: Vec<MoveInventoryItem>,
    pub quote: Option<MoveQuote>,
    pub db_trigger: Signal<u32>,
}

pub fn trigger_download(
    toast: &dioxus_primitives::toast::Toasts,
    locale: &str,
    content: &str,
    file_name: &str,
) {
    #[cfg(target_arch = "wasm32")]
    {
        toast.info(
            t("school-toast-download-started", locale),
            dioxus_primitives::toast::ToastOptions::new()
                .description(t("school-toast-browser-download-desc", locale)),
        );

        let base64_str = crate::views::school::academics::utils::base64_encode(content.as_bytes());
        let js_code = format!(
            r#"
            (function() {{
                const base64 = "{}";
                const filename = "{}";
                const binString = atob(base64);
                const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                const blob = new Blob([bytes], {{ type: "application/octet-stream" }});
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = filename;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);
            }})();
            "#,
            base64_str, file_name
        );
        let _ = js_sys::eval(&js_code);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let file_path = rfd::FileDialog::new().set_file_name(file_name).save_file();
        if let Some(path) = file_path {
            if std::fs::write(&path, content).is_ok() {
                let desc = format!("{} {}", t("school-toast-saved-to", locale), path.display());
                toast.success(
                    t("school-toast-export-success", locale),
                    dioxus_primitives::toast::ToastOptions::new().description(desc),
                );
            } else {
                toast.error(
                    t("school-toast-export-failed", locale),
                    dioxus_primitives::toast::ToastOptions::new()
                        .description(t("school-toast-export-failed-desc", locale)),
                );
            }
        }
    }
}
