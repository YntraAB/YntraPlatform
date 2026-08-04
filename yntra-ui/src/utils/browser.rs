use dioxus_primitives::toast::{ToastOptions, Toasts};

pub fn open_in_system_browser(_url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(&["/C", "start", "", _url])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(_url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(_url).spawn();
}

/// Centralized copy utility capturing clipboard promises and dispatching success/error toast alerts.
pub fn copy_to_clipboard(text: String, toast: Option<Toasts>) {
    let text_json = serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".to_string());
    let js = format!(
        r#"
        if (navigator.clipboard && navigator.clipboard.writeText) {{
            navigator.clipboard.writeText({})
                .then(() => {{ dioxus.send(JSON.stringify({{ success: true }})); }})
                .catch((err) => {{ dioxus.send(JSON.stringify({{ success: false, error: String(err) }})); }});
        }} else {{
            try {{
                let textArea = document.createElement("textarea");
                textArea.value = {};
                textArea.style.position = "fixed";
                textArea.style.opacity = "0";
                document.body.appendChild(textArea);
                textArea.focus();
                textArea.select();
                let successful = document.execCommand('copy');
                document.body.removeChild(textArea);
                dioxus.send(JSON.stringify({{ success: successful }}));
            }} catch (err) {{
                dioxus.send(JSON.stringify({{ success: false, error: String(err) }}));
            }}
        }}
        "#,
        text_json, text_json
    );

    let mut eval = dioxus::document::eval(&js);
    dioxus::prelude::spawn(async move {
        let res = eval.recv::<serde_json::Value>().await;
        let is_success = match &res {
            Ok(val) => val
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            Err(_) => false,
        };

        if let Some(t_api) = toast {
            if is_success {
                t_api.success(
                    "Copied to Clipboard".to_string(),
                    ToastOptions::new().description("Content copied to clipboard successfully."),
                );
            } else {
                let err_detail = res
                    .ok()
                    .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                    .unwrap_or_else(|| "Failed to write to clipboard.".to_string());
                t_api.error(
                    "Copy Failed".to_string(),
                    ToastOptions::new().description(err_detail),
                );
            }
        }
    });
}

pub fn get_anon_report_ids() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                if let Ok(Some(json)) = storage.get_item("yntra_anon_report_ids") {
                    if let Ok(ids) = serde_json::from_str::<Vec<String>>(&json) {
                        return ids;
                    }
                }
            }
        }
    }
    Vec::new()
}
