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

#[component]
pub fn LucideIcon(props: LucideIconProps) -> Element {
    let size_val = props.size.unwrap_or_else(|| "18".to_string());
    let size = size_val.as_str();
    let color_val = props.color.unwrap_or_else(|| "currentColor".to_string());
    let color = color_val.as_str();
    let class_val = props.class.unwrap_or_default();
    let class = class_val.as_str();

    let svg_content = match props.name.as_str() {
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
        "book-open" => rsx! {
            path { d: "M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z" }
            path { d: "M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z" }
        },
        "graduation-cap" | "school" => rsx! {
            path { d: "M21.42 10.922a1 1 0 0 0-.019-1.838L12.83 5.18a2 2 0 0 0-1.66 0L2.6 9.08a1 1 0 0 0 0 1.832l8.57 3.908a2 2 0 0 0 1.66 0z" }
            path { d: "M6 12v5c0 2 2 3 6 3s6-1 6-3v-5" }
            path { d: "M21.5 12v6" }
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
