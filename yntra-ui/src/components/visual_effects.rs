use dioxus::document::eval;
use dioxus::prelude::*;
use serde_json::Value;

#[derive(Props, Clone, PartialEq)]
pub struct VisualEffectHandlerProps {
    pub account_preferences: Signal<String>,
    pub workspace_brand_color: String,
}

fn hex_to_hsl(hex: &str) -> Option<(u16, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 && hex.len() != 3 {
        return None;
    }
    let (r, g, b) = if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
        (r, g, b)
    } else {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        (r, g, b)
    };

    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);

    let l = (max + min) / 2.0;
    let mut h = 0.0;
    let mut s = 0.0;

    if max != min {
        let d = max - min;
        s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        if max == r {
            h = (g - b) / d + (if g < b { 6.0 } else { 0.0 });
        } else if max == g {
            h = (b - r) / d + 2.0;
        } else {
            h = (r - g) / d + 4.0;
        }
        h /= 6.0;
    }

    Some((
        (h * 360.0).round() as u16,
        (s * 100.0).round() as u8,
        (l * 100.0).round() as u8,
    ))
}

#[component]
pub fn VisualEffectHandler(props: VisualEffectHandlerProps) -> Element {
    use_effect(move || {
        let prefs_str = props.account_preferences.read();

        // Parse preferences
        let mut accent_color = props.workspace_brand_color.clone();
        let mut theme_mode = "dark".to_string();

        if let Ok(val) = serde_json::from_str::<Value>(&prefs_str) {
            let accent_key = val
                .get("accent")
                .or_else(|| val.get("accent_color"))
                .and_then(|v| v.as_str());
            if let Some(color) = accent_key {
                if color != "primary" && !color.is_empty() {
                    accent_color = color.to_string();
                }
            }
            if let Some(theme) = val.get("theme").and_then(|v| v.as_str()) {
                theme_mode = theme.to_string();
            }
        }

        let mut js = format!(
            r#"
            document.documentElement.className = "{}";
            document.documentElement.setAttribute("data-theme", "{}");
            "#,
            theme_mode, theme_mode
        );

        if let Some((h, s, l)) = hex_to_hsl(&accent_color) {
            let primary_coords = format!("{} {}% {}%", h, s, l);
            let foreground_coords = if l > 60 { "0 0% 0%" } else { "0 0% 100%" };
            let accent_color_str = format!("hsl({} {}% {}%)", h, s, l);
            let primary_foreground_color_str = if l > 60 {
                "hsl(0 0% 0%)"
            } else {
                "hsl(0 0% 100%)"
            };

            let hover_l = if l > 50 {
                l.saturating_sub(10)
            } else {
                l.saturating_add(10)
            };
            let accent_color_hover_str = format!("hsl({} {}% {}%)", h, s, hover_l);
            let accent_color_soft_str = format!("hsla({}, {}%, {}%, 0.15)", h, s, l);

            js.push_str(&format!(
                r#"
                document.documentElement.style.setProperty('--primary', '{}');
                document.documentElement.style.setProperty('--ring', '{}');
                document.documentElement.style.setProperty('--primary-foreground', '{}');
                document.documentElement.style.setProperty('--primary-foreground-color', '{}');
                document.documentElement.style.setProperty('--accent-color', '{}');
                document.documentElement.style.setProperty('--accent-color-hover', '{}');
                document.documentElement.style.setProperty('--accent-color-soft', '{}');
                document.documentElement.style.setProperty('--focused-border-color', '{}');
                "#,
                primary_coords,
                primary_coords,
                foreground_coords,
                primary_foreground_color_str,
                accent_color_str,
                accent_color_hover_str,
                accent_color_soft_str,
                accent_color_str
            ));
        } else {
            js.push_str(&format!(
                r#"
                document.documentElement.style.setProperty('--accent-color', '{}');
                document.documentElement.style.setProperty('--accent-color-hover', '{}');
                document.documentElement.style.setProperty('--accent-color-soft', 'rgba(99, 102, 241, 0.15)');
                document.documentElement.style.setProperty('--focused-border-color', '{}');
                "#,
                accent_color, accent_color, accent_color
            ));
        }

        let _ = eval(&js);
    });

    let prefs_str = props.account_preferences.read();
    let mut font_scale = 100.0f32;
    let mut theme_mode = "dark".to_string();

    if let Ok(val) = serde_json::from_str::<Value>(&prefs_str) {
        if let Some(scale) = val.get("font_scale").and_then(|v| v.as_f64()) {
            let scale_f32 = scale as f32;
            if scale_f32 > 5.0 {
                font_scale = scale_f32;
            } else {
                font_scale = scale_f32 * 100.0;
            }
        }
        if let Some(theme) = val.get("theme").and_then(|v| v.as_str()) {
            theme_mode = theme.to_string();
        }
    }

    rsx! {
        style {
            {
                format!(
                    r#"
                    :root {{
                        font-size: {}%;
                    }}
                    body {{
                        color-scheme: {};
                    }}
                    "#,
                    font_scale,
                    theme_mode,
                )
            }
        }
    }
}
