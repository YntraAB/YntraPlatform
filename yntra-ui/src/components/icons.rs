use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct LucideIconProps {
    #[props(into)]
    pub name: String,
    #[props(optional, into)]
    pub size: Option<String>,
    #[props(optional, into)]
    pub color: Option<String>,
    #[props(optional, into)]
    pub class: Option<String>,
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut last_was_dash = false;
    for (i, c) in s.chars().enumerate() {
        if c == '_' || c == '-' || c == ' ' {
            if !last_was_dash && !result.is_empty() {
                result.push('-');
                last_was_dash = true;
            }
        } else if c.is_uppercase() {
            if i > 0 && !last_was_dash {
                result.push('-');
            }
            result.extend(c.to_lowercase());
            last_was_dash = false;
        } else {
            result.push(c);
            last_was_dash = false;
        }
    }
    if result.ends_with('-') {
        result.pop();
    }
    result
}

#[component]
pub fn LucideIcon(props: LucideIconProps) -> Element {
    let size_val = props.size.unwrap_or_else(|| "18".to_string());
    let size = size_val.as_str();
    let color_val = props.color.unwrap_or_else(|| "currentColor".to_string());
    let color = color_val.as_str();
    let class_val = props.class.unwrap_or_default();
    let class = class_val.as_str();

    let name_kebab = to_kebab_case(&props.name);
    let svg_content = match name_kebab.as_str() {
        "layout-dashboard" | "dashboard" => rsx! {
            rect {
                x: "3",
                y: "3",
                width: "7",
                height: "9",
                rx: "1",
            }
            rect {
                x: "14",
                y: "3",
                width: "7",
                height: "5",
                rx: "1",
            }
            rect {
                x: "14",
                y: "12",
                width: "7",
                height: "9",
                rx: "1",
            }
            rect {
                x: "3",
                y: "16",
                width: "7",
                height: "5",
                rx: "1",
            }
        },
        "message-square" | "messaging" => rsx! {
            path { d: "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" }
        },
        "calendar" | "scheduling" => rsx! {
            rect {
                x: "3",
                y: "4",
                width: "18",
                height: "18",
                rx: "2",
                ry: "2",
            }
            line {
                x1: "16",
                y1: "2",
                x2: "16",
                y2: "6",
            }
            line {
                x1: "8",
                y1: "2",
                x2: "8",
                y2: "6",
            }
            line {
                x1: "3",
                y1: "10",
                x2: "21",
                y2: "10",
            }
        },
        "file-text" | "notes" => rsx! {
            path { d: "M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" }
            polyline { points: "14 2 14 8 20 8" }
            line {
                x1: "16",
                y1: "13",
                x2: "8",
                y2: "13",
            }
            line {
                x1: "16",
                y1: "17",
                x2: "8",
                y2: "17",
            }
            line {
                x1: "10",
                y1: "9",
                x2: "8",
                y2: "9",
            }
        },
        "clock" | "time" => rsx! {
            circle { cx: "12", cy: "12", r: "10" }
            polyline { points: "12 6 12 12 16 14" }
        },
        "pill" => rsx! {
            path { d: "m10.5 20.5 10-10a4.95 4.95 0 1 0-7-7l-10 10a4.95 4.95 0 1 0 7 7Z" }
            path { d: "m8.5 8.5 7 7" }
        },
        "book-open" | "library" | "book-copy" => rsx! {
            path { d: "M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z" }
            path { d: "M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z" }
        },
        "graduation-cap" | "school" | "academics" => rsx! {
            path { d: "M21.42 10.922a1 1 0 0 0-.019-1.838L12.83 5.18a2 2 0 0 0-1.66 0L2.6 9.08a1 1 0 0 0 0 1.832l8.57 3.908a2 2 0 0 0 1.66 0z" }
            path { d: "M6 12v5c0 2 2 3 6 3s6-1 6-3v-5" }
            path { d: "M21.5 12v6" }
        },
        "link" => rsx! {
            path { d: "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" }
            path { d: "M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" }
        },
        "user-cog" => rsx! {
            circle { cx: "9", cy: "7", r: "4" }
            path { d: "M10 15H9a7 7 0 0 0-7 7" }
            circle { cx: "19", cy: "16", r: "1" }
            path { d: "m19 13-.3 1.1c-.2.1-.4.2-.6.3l-1-.5-.7.7.5 1c-.1.2-.2.4-.3.6l-1.1.3v1l1.1.3c.1.2.2.4.3.6l-.5 1 .7.7 1-.5c.2.1.4.2.6.3l.3 1.1h1l.3-1.1c.2-.1.4-.2.6-.3l1 .5.7-.7-.5-1c.1-.2.2-.4.3-.6l1.1-.3v-1l-1.1-.3c-.1-.2-.2-.4-.3-.6l.5-1-.7-.7-1 .5c-.2-.1-.4-.2-.6-.3Z" }
        },
        "shield-alert" => rsx! {
            path { d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1z" }
            line { x1: "12", y1: "8", x2: "12", y2: "12" }
            line { x1: "12", y1: "16", x2: "12.01", y2: "16" }
        },
        "download" => rsx! {
            path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
            polyline { points: "7 10 12 15 17 10" }
            line { x1: "12", y1: "15", x2: "12", y2: "3" }
        },
        "upload-cloud" => rsx! {
            path { d: "M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242" }
            path { d: "M12 12v9" }
            path { d: "m16 16-4-4-4 4" }
        },
        "send" => rsx! {
            line { x1: "22", y1: "2", x2: "11", y2: "13" }
            polygon { points: "22 2 15 22 11 13 2 9 22 2" }
        },
        "copy" => rsx! {
            rect { width: "14", height: "14", x: "8", y: "8", rx: "2", ry: "2" }
            path { d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" }
        },
        "activity" => rsx! {
            polyline { points: "22 12 18 12 15 21 9 3 6 12 2 12" }
        },

        "heart" | "assistance" | "client_portal" => rsx! {
            path { d: "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" }
        },
        "folder-open" | "directory" => rsx! {
            path { d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
            path { d: "M2 10h20" }
        },
        "settings" => rsx! {
            path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.1a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
            circle { cx: "12", cy: "12", r: "3" }
        },
        "alert-triangle" | "reporting" => rsx! {
            path { d: "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" }
            line {
                x1: "12",
                y1: "9",
                x2: "12",
                y2: "13",
            }
            line {
                x1: "12",
                y1: "17",
                x2: "12.01",
                y2: "17",
            }
        },
        "chevron-left" => rsx! {
            polyline { points: "15 18 9 12 15 6" }
        },
        "chevron-right" => rsx! {
            polyline { points: "9 18 15 12 9 6" }
        },
        "chevron-down" => rsx! {
            polyline { points: "6 9 12 15 18 9" }
        },
        "wifi-off" => rsx! {
            line {
                x1: "1",
                y1: "1",
                x2: "23",
                y2: "23",
            }
            path { d: "M16.72 11.06A10.94 10.94 0 0 1 19 12.5" }
            path { d: "M5 12.5a10.94 10.94 0 0 1 5.83-2.84" }
            path { d: "M12 18.5a4.25 4.25 0 0 1-1.57-.3" }
            path { d: "M12.9 6.2a15 15 0 0 1 6.1 1.8" }
            path { d: "M1.62 7.82a15 15 0 0 1 17.65-1.56" }
        },
        "refresh-cw" => rsx! {
            polyline { points: "23 4 23 10 17 10" }
            polyline { points: "1 20 1 14 7 14" }
            path { d: "M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" }
        },
        "alert-circle" => rsx! {
            circle { cx: "12", cy: "12", r: "10" }
            line {
                x1: "12",
                y1: "8",
                x2: "12",
                y2: "12",
            }
            line {
                x1: "12",
                y1: "16",
                x2: "12.01",
                y2: "16",
            }
        },
        "languages" | "translate" => rsx! {
            path { d: "m5 8 6 6" }
            path { d: "m4 14 6-6 2-3" }
            path { d: "M2 5h12" }
            path { d: "M7 2h1" }
            path { d: "m22 22-5-10-5 10" }
            path { d: "M14 18h6" }
        },
        "sun" => rsx! {
            circle { cx: "12", cy: "12", r: "4" }
            path { d: "M12 2v2" }
            path { d: "M12 20v2" }
            path { d: "m4.93 4.93 1.41 1.41" }
            path { d: "m17.66 17.66 1.41 1.41" }
            path { d: "M2 12h2" }
            path { d: "M20 12h2" }
            path { d: "m6.34 17.66-1.41 1.41" }
            path { d: "m19.07 4.93-1.41 1.41" }
        },
        "moon" => rsx! {
            path { d: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" }
        },
        "more-vertical" => rsx! {
            circle { cx: "12", cy: "12", r: "1" }
            circle { cx: "12", cy: "5", r: "1" }
            circle { cx: "12", cy: "19", r: "1" }
        },
        "more-horizontal" => rsx! {
            circle { cx: "12", cy: "12", r: "1" }
            circle { cx: "19", cy: "12", r: "1" }
            circle { cx: "5", cy: "12", r: "1" }
        },
        "zap" => rsx! {
            polygon { points: "13 2 3 14 12 14 11 22 21 10 12 10 13 2" }
        },
        "shield" => rsx! {
            path { d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1z" }
        },
        "shield-check" => rsx! {
            path { d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1z" }
            path { d: "m9 12 2 2 4-4" }
        },
        "leaf" => rsx! {
            path { d: "M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 3.58 1 8a7 7 0 0 1-9 10Z" }
            path { d: "M9 22c0-3 1.7-4.7 5-5" }
        },
        "monitor" => rsx! {
            rect { width: "20", height: "14", x: "2", y: "3", rx: "2" }
            line { x1: "8", x2: "16", y1: "21", y2: "21" }
            line { x1: "12", x2: "12", y1: "17", y2: "21" }
        },
        "user" => rsx! {
            path { d: "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" }
            circle { cx: "12", cy: "7", r: "4" }
        },
        "palette" => rsx! {
            path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.1a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
            circle { cx: "12", cy: "12", r: "3" }
        },
        "check-square" => rsx! {
            rect { x: "3", y: "3", width: "18", height: "18", rx: "2", ry: "2" }
            polyline { points: "9 11 12 14 17 9" }
        },
        "check" => rsx! {
            polyline { points: "20 6 9 17 4 12" }
        },
        "upload" => rsx! {
            path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
            polyline { points: "17 8 12 3 7 8" }
            line { x1: "12", x2: "12", y1: "3", y2: "15" }
        },
        "globe" => rsx! {
            circle { cx: "12", cy: "12", r: "10" }
            line { x1: "2", x2: "22", y1: "12", y2: "12" }
            path { d: "M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" }
        },
        "x" => rsx! {
            line { x1: "18", x2: "6", y1: "6", y2: "18" }
            line { x1: "6", x2: "18", y1: "6", y2: "18" }
        },
        "mail" => rsx! {
            rect { width: "20", height: "14", x: "2", y: "5", rx: "2" }
            path { d: "m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7" }
        },
        "bell" => rsx! {
            path { d: "M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" }
            path { d: "M10.3 21a1.94 1.94 0 0 0 3.4 0" }
        },
        "bell-ring" => rsx! {
            path { d: "M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" }
            path { d: "M10.3 21a1.94 1.94 0 0 0 3.4 0" }
            path { d: "M4 2C2.8 3.7 2.1 5.7 2.1 8" }
            path { d: "M20 2c1.2 1.7 1.9 3.7 1.9 6" }
        },
        "search" => rsx! {
            circle { cx: "11", cy: "11", r: "8" }
            line { x1: "21", x2: "16.65", y1: "21", y2: "16.65" }
        },
        "settings-2" => rsx! {
            path { d: "M20 7h-9" }
            path { d: "M14 17H5" }
            circle { cx: "17", cy: "17", r: "3" }
            circle { cx: "7", cy: "7", r: "3" }
        },
        "loader-2" => rsx! {
            path { d: "M21 12a9 9 0 1 1-6.219-8.56" }
        },
        "key" => rsx! {
            path { d: "m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.778-7.778zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3m-3.5 3.5L19 4" }
        },
        "arrow-left" => rsx! {
            path { d: "m12 19-7-7 7-7" }
            path { d: "M19 12H5" }
        },
        "arrow-right" => rsx! {
            path { d: "M5 12h14" }
            path { d: "m12 5 7 7-7 7" }
        },
        "arrow_right" => rsx! {
            path { d: "M5 12h14" }
            path { d: "m12 5 7 7-7 7" }
        },
        "award" => rsx! {
            path { d: "m15.477 12.89 1.515 8.526a.5.5 0 0 1-.81.47l-3.58-2.687a1 1 0 0 0-1.197 0l-3.586 2.686a.5.5 0 0 1-.81-.469l1.514-8.526" }
            circle { cx: "12", cy: "8", r: "6" }
        },
        "box" => rsx! {
            path { d: "M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z" }
            path { d: "m3.3 7 8.7 5 8.7-5" }
            path { d: "M12 22V12" }
        },
        "building-2" => rsx! {
            path { d: "M10 12h4" }
            path { d: "M10 8h4" }
            path { d: "M14 21v-3a2 2 0 0 0-4 0v3" }
            path { d: "M6 10H4a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-2" }
            path { d: "M6 21V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v16" }
        },
        "circle" => rsx! {
            circle { cx: "12", cy: "12", r: "10" }
        },
        "credit-card" => rsx! {
            rect { width: "20", height: "14", x: "2", y: "5", rx: "2" }
            line { x1: "2", x2: "22", y1: "10", y2: "10" }
        },
        "external-link" => rsx! {
            path { d: "M15 3h6v6" }
            path { d: "M10 14 21 3" }
            path { d: "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" }
        },
        "file-check" => rsx! {
            path { d: "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z" }
            path { d: "M14 2v5a1 1 0 0 0 1 1h5" }
            path { d: "m9 15 2 2 4-4" }
        },
        "image" => rsx! {
            rect { width: "18", height: "18", x: "3", y: "3", rx: "2", ry: "2" }
            circle { cx: "9", cy: "9", r: "2" }
            path { d: "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" }
        },
        "inbox" => rsx! {
            polyline { points: "22 12 16 12 14 15 10 15 8 12 2 12" }
            path { d: "M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" }
        },
        "layout-grid" => rsx! {
            rect { width: "7", height: "7", x: "3", y: "3", rx: "1" }
            rect { width: "7", height: "7", x: "14", y: "3", rx: "1" }
            rect { width: "7", height: "7", x: "14", y: "14", rx: "1" }
            rect { width: "7", height: "7", x: "3", y: "14", rx: "1" }
        },
        "life-buoy" => rsx! {
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "m4.93 4.93 4.24 4.24" }
            path { d: "m14.83 9.17 4.24-4.24" }
            path { d: "m14.83 14.83 4.24 4.24" }
            path { d: "m9.17 14.83-4.24 4.24" }
            circle { cx: "12", cy: "12", r: "4" }
        },
        "lock" => rsx! {
            rect { width: "18", height: "11", x: "3", y: "11", rx: "2", ry: "2" }
            path { d: "M7 11V7a5 5 0 0 1 10 0v4" }
        },
        "logout" | "log-out" => rsx! {
            path { d: "m16 17 5-5-5-5" }
            path { d: "M21 12H9" }
            path { d: "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" }
        },
        "map-pin" => rsx! {
            path { d: "M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0" }
            circle { cx: "12", cy: "10", r: "3" }
        },
        "navigation" => rsx! {
            polygon { points: "3 11 22 2 13 21 11 13 3 11" }
        },
        "package" => rsx! {
            path { d: "M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z" }
            path { d: "M12 22V12" }
            polyline { points: "3.29 7 12 12 20.71 7" }
            path { d: "m7.5 4.27 9 5.15" }
        },
        "pen-tool" => rsx! {
            path { d: "M15.707 21.293a1 1 0 0 1-1.414 0l-1.586-1.586a1 1 0 0 1 0-1.414l5.586-5.586a1 1 0 0 1 1.414 0l1.586 1.586a1 1 0 0 1 0 1.414z" }
            path { d: "m18 13-1.375-6.874a1 1 0 0 0-.746-.776L3.235 2.028a1 1 0 0 0-1.207 1.207L5.35 15.879a1 1 0 0 0 .776.746L13 18" }
            path { d: "m2.3 2.3 7.286 7.286" }
            circle { cx: "11", cy: "11", r: "2" }
        },
        "phone" => rsx! {
            path { d: "M13.832 16.568a1 1 0 0 0 1.213-.303l.355-.465A2 2 0 0 1 17 15h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2A18 18 0 0 1 2 4a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v3a2 2 0 0 1-.8 1.6l-.468.351a1 1 0 0 0-.292 1.233 14 14 0 0 0 6.392 6.384" }
        },
        "play" => rsx! {
            path { d: "M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z" }
        },
        "plus" => rsx! {
            path { d: "M5 12h14" }
            path { d: "M12 5v14" }
        },
        "printer" => rsx! {
            path { d: "M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" }
            path { d: "M6 9V3a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v6" }
            rect { x: "6", y: "14", width: "12", height: "8", rx: "1" }
        },
        "reply" => rsx! {
            path { d: "M20 18v-2a4 4 0 0 0-4-4H4" }
            path { d: "m9 17-5-5 5-5" }
        },
        "rocket" => rsx! {
            path { d: "M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5" }
            path { d: "M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09" }
            path { d: "M9 12a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.4 22.4 0 0 1-4 2z" }
            path { d: "M9 12H4s.55-3.03 2-4c1.62-1.08 5 .05 5 .05" }
        },
        "save" => rsx! {
            path { d: "M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z" }
            path { d: "M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7" }
            path { d: "M7 3v4a1 1 0 0 0 1 1h7" }
        },
        "slash" => rsx! {
            path { d: "M22 2 2 22" }
        },
        "smartphone" => rsx! {
            rect { width: "14", height: "20", x: "5", y: "2", rx: "2", ry: "2" }
            path { d: "M12 18h.01" }
        },
        "sparkles" => rsx! {
            path { d: "M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z" }
            path { d: "M20 2v4" }
            path { d: "M22 4h-4" }
            circle { cx: "4", cy: "20", r: "2" }
        },
        "star" => rsx! {
            path { d: "M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z" }
        },
        "tag" => rsx! {
            path { d: "M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z" }
            circle { cx: "7.5", cy: "7.5", r: ".5", fill: "currentColor" }
        },
        "thumbs-up" => rsx! {
            path { d: "M15 5.88 14 10h5.83a2 2 0 0 1 1.92 2.56l-2.33 8A2 2 0 0 1 17.5 22H4a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h2.76a2 2 0 0 0 1.79-1.11L12 2a3.13 3.13 0 0 1 3 3.88Z" }
            path { d: "M7 10v12" }
        },
        "trash" => rsx! {
            path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" }
            path { d: "M3 6h18" }
            path { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
        },
        "trash-2" => rsx! {
            path { d: "M10 11v6" }
            path { d: "M14 11v6" }
            path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" }
            path { d: "M3 6h18" }
            path { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
        },
        "trophy" => rsx! {
            path { d: "M10 14.66v1.626a2 2 0 0 1-.976 1.696A5 5 0 0 0 7 21.978" }
            path { d: "M14 14.66v1.626a2 2 0 0 0 .976 1.696A5 5 0 0 1 17 21.978" }
            path { d: "M18 9h1.5a1 1 0 0 0 0-5H18" }
            path { d: "M4 22h16" }
            path { d: "M6 9a6 6 0 0 0 12 0V3a1 1 0 0 0-1-1H7a1 1 0 0 0-1 1z" }
            path { d: "M6 9H4.5a1 1 0 0 1 0-5H6" }
        },
        "truck" => rsx! {
            path { d: "M14 18V6a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2v11a1 1 0 0 0 1 1h2" }
            path { d: "M15 18H9" }
            path { d: "M19 18h2a1 1 0 0 0 1-1v-3.65a1 1 0 0 0-.22-.624l-3.48-4.35A1 1 0 0 0 17.52 8H14" }
            circle { cx: "17", cy: "18", r: "2" }
            circle { cx: "7", cy: "18", r: "2" }
        },
        "user-check" => rsx! {
            path { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" }
            circle { cx: "9", cy: "7", r: "4" }
            polyline { points: "16 11 18 13 22 9" }
        },
        "users" => rsx! {
            path { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" }
            path { d: "M16 3.128a4 4 0 0 1 0 7.744" }
            path { d: "M22 21v-2a4 4 0 0 0-3-3.87" }
            circle { cx: "9", cy: "7", r: "4" }
        },
        "wallet" => rsx! {
            path { d: "M19 7V4a1 1 0 0 0-1-1H5a2 2 0 0 0 0 4h15a1 1 0 0 1 1 1v4h-3a2 2 0 0 0 0 4h3a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1" }
            path { d: "M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4" }
        },
        "wrench" => rsx! {
            path { d: "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.106-3.105c.32-.322.863-.22.983.218a6 6 0 0 1-8.259 7.057l-7.91 7.91a1 1 0 0 1-2.999-3l7.91-7.91a6 6 0 0 1 7.057-8.259c.438.12.54.662.219.984z" }
        },

        "align-left" => rsx! {
            path { d: "M21 5H3" }
            path { d: "M15 12H3" }
            path { d: "M17 19H3" }
        },
        "check-circle" => rsx! {
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "m9 12 2 2 4-4" }
        },
        "edit" => rsx! {
            path { d: "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" }
            path { d: "m15 5 4 4" }
        },
        "file-edit" => rsx! {
            path { d: "M12.659 22H18a2 2 0 0 0 2-2V8a2.4 2.4 0 0 0-.706-1.706l-3.588-3.588A2.4 2.4 0 0 0 14 2H6a2 2 0 0 0-2 2v9.34" }
            path { d: "M14 2v5a1 1 0 0 0 1 1h5" }
            path { d: "M10.378 12.622a1 1 0 0 1 3 3.003L8.36 20.637a2 2 0 0 1-.854.506l-2.867.837a.5.5 0 0 1-.62-.62l.836-2.869a2 2 0 0 1 .506-.853z" }
        },
        "home" => rsx! {
            path { d: "M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8" }
            path { d: "M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }
        },
        "layout" => rsx! {
            rect { width: "18", height: "7", x: "3", y: "3", rx: "1" }
            rect { width: "9", height: "7", x: "3", y: "14", rx: "1" }
            rect { width: "5", height: "7", x: "16", y: "14", rx: "1" }
        },
        "sliders" => rsx! {
            path { d: "M10 5H3" }
            path { d: "M12 19H3" }
            path { d: "M14 3v4" }
            path { d: "M16 17v4" }
            path { d: "M21 12h-9" }
            path { d: "M21 19h-5" }
            path { d: "M21 5h-7" }
            path { d: "M8 10v4" }
            path { d: "M8 12H3" }
        },
        _ => rsx! {
            circle { cx: "12", cy: "12", r: "10" }
        },
    };

    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "{color}",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            class: "{class}",
            {svg_content}
        }
    }
}
