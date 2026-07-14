use dioxus::prelude::*;
use qrcodegen::{QrCode, QrCodeEcc};

pub fn render_qr_svg(data: &str) -> Element {
    if let Ok(qr) = QrCode::encode_text(data, QrCodeEcc::Medium) {
        let size = qr.size();
        let border = 2;
        let total_size = size + border * 2;

        let mut rects = Vec::new();
        for y in 0..size {
            for x in 0..size {
                if qr.get_module(x, y) {
                    rects.push(rsx! {
                        rect {
                            key: "{x}-{y}",
                            x: x + border,
                            y: y + border,
                            width: 1,
                            height: 1,
                            fill: "#000000",
                        }
                    });
                }
            }
        }

        rsx! {
            svg {
                view_box: "0 0 {total_size} {total_size}",
                style: "width: 100%; height: 100%; background: #ffffff; border-radius: 4px; padding: 4px; box-sizing: border-box;",
                rect {
                    width: total_size,
                    height: total_size,
                    fill: "#ffffff",
                }
                {rects.into_iter()}
            }
        }
    } else {
        rsx! {
            div { style: "color: var(--danger); font-size: 11px;", "Error generating QR" }
        }
    }
}
