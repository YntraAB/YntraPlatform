#[allow(unused_variables)]
pub fn trigger_print_or_pdf_download(
    toast: &dioxus_primitives::toast::Toasts,
    html_content: &str,
    document_name: &str,
) {
    #[cfg(target_arch = "wasm32")]
    {
        toast.info(
            "Opening Print / PDF Export Preview...".to_string(),
            dioxus_primitives::toast::ToastOptions::new(),
        );
        let base64_str = base64_encode_str(html_content.as_bytes());
        let js_code = format!(
            r#"
            (function() {{
                const base64 = "{}";
                const binString = atob(base64);
                const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                const blob = new Blob([bytes], {{ type: "text/html;charset=utf-8" }});
                const url = URL.createObjectURL(blob);
                const printWin = window.open(url, "_blank");
                if (printWin) {{
                    printWin.onload = function() {{
                        printWin.focus();
                        printWin.print();
                    }};
                }} else {{
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = "{}";
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    URL.revokeObjectURL(url);
                }}
            }})();
            "#,
            base64_str, document_name
        );
        let _ = dioxus::document::eval(&js_code);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let file_path = rfd::FileDialog::new()
            .set_file_name(document_name)
            .save_file();
        if let Some(path) = file_path {
            if std::fs::write(&path, html_content).is_ok() {
                toast.success(
                    format!("Saved printable PDF document to {}", path.display()),
                    dioxus_primitives::toast::ToastOptions::new(),
                );
            } else {
                toast.error(
                    "Failed to save printable document".to_string(),
                    dioxus_primitives::toast::ToastOptions::new(),
                );
            }
        }
    }
}

#[allow(dead_code)]
fn base64_encode_str(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as u32
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        encoded.push(CHARS[((triple >> 18) & 63) as usize] as char);
        encoded.push(CHARS[((triple >> 12) & 63) as usize] as char);
        if i + 1 < data.len() {
            encoded.push(CHARS[((triple >> 6) & 63) as usize] as char);
        } else {
            encoded.push('=');
        }
        if i + 2 < data.len() {
            encoded.push(CHARS[(triple & 63) as usize] as char);
        } else {
            encoded.push('=');
        }
        i += 3;
    }
    encoded
}
