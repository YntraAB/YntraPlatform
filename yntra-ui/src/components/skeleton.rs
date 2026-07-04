use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SkeletonProps {
    #[props(default = String::new())]
    pub width: String,
    #[props(default = String::new())]
    pub height: String,
    #[props(default = String::new())]
    pub class: String,
}

#[component]
pub fn Skeleton(props: SkeletonProps) -> Element {
    let mut style_attr = String::new();
    if !props.width.is_empty() {
        style_attr.push_str(&format!("width: {}; ", props.width));
    }
    if !props.height.is_empty() {
        style_attr.push_str(&format!("height: {}; ", props.height));
    }

    rsx! {
        div { class: "yntra-skeleton {props.class}", style: "{style_attr}",
            style {
                r#"
                .yntra-skeleton {{
                    background: linear-gradient(90deg, rgba(255, 255, 255, 0.05) 25%, rgba(255, 255, 255, 0.1) 37%, rgba(255, 255, 255, 0.05) 63%);
                    background-size: 400% 100%;
                    animation: skeleton-loading 1.4s ease infinite;
                    border-radius: 6px;
                    display: inline-block;
                    min-height: 1rem;
                    width: 100%;
                }}
                @keyframes skeleton-loading {{
                    0% {{ background-position: 100% 50%; }}
                    100% {{ background-position: 0% 50%; }}
                }}
                "#
            }
        }
    }
}
