use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct TooltipProps {
    pub text: String,
    pub children: Element,
}

#[component]
pub fn Tooltip(props: TooltipProps) -> Element {
    rsx! {
        div { class: "yntra-tooltip-wrapper",
            style {
                r#"
                .yntra-tooltip-wrapper {{
                    position: relative;
                    display: inline-block;
                }}
                .yntra-tooltip-text {{
                    visibility: hidden;
                    background: var(--primary-color-4);
                    border: 1px solid var(--primary-color-6);
                    color: var(--secondary-color-2);
                    text-align: center;
                    padding: 0.4rem 0.8rem;
                    border-radius: 6px;
                    position: absolute;
                    bottom: 125%;
                    left: 50%;
                    transform: translateX(-50%);
                    opacity: 0;
                    transition: opacity 0.2s;
                    font-size: 0.8rem;
                    white-space: nowrap;
                    z-index: 200;
                    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.3);
                }}
                .yntra-tooltip-wrapper:hover .yntra-tooltip-text {{
                    visibility: visible;
                    opacity: 1;
                }}
                "#
            }
            {props.children}
            span { class: "yntra-tooltip-text", "{props.text}" }
        }
    }
}
