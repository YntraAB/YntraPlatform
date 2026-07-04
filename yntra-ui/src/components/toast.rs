use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ToastProps {
    pub message: String,
    pub open: bool,
    pub onclose: EventHandler<()>,
    #[props(default = 3000)]
    pub duration_ms: u32,
}

#[component]
pub fn Toast(props: ToastProps) -> Element {
    if !props.open {
        return rsx! {};
    }

    // Auto-close effect
    use_effect(move || {
        let duration = props.duration_ms;
        spawn(async move {
            crate::utils::sleep_ms(duration).await;
            props.onclose.call(());
        });
    });

    rsx! {
        div { class: "yntra-toast-container",
            style {
                r#"
                .yntra-toast-container {{
                    position: fixed;
                    bottom: 24px;
                    right: 24px;
                    background: var(--primary-color-2);
                    backdrop-filter: blur(12px);
                    border: 1px solid var(--primary-color-6);
                    border-left: 4px solid var(--focused-border-color);
                    border-radius: 10px;
                    padding: 1rem 1.5rem;
                    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
                    z-index: 2000;
                    display: flex;
                    align-items: center;
                    gap: 1rem;
                    animation: slideIn 0.3s ease;
                }}
                .yntra-toast-message {{
                    color: var(--secondary-color-2);
                    font-weight: 500;
                    font-size: 0.95rem;
                }}
                .yntra-toast-close {{
                    background: none;
                    border: none;
                    color: var(--secondary-color-5);
                    cursor: pointer;
                    font-size: 1rem;
                }}
                @keyframes slideIn {{
                    from {{ transform: translateY(100px); opacity: 0; }}
                    to {{ transform: translateY(0); opacity: 1; }}
                }}
                "#
            }
            span { class: "yntra-toast-message", "{props.message}" }
            button {
                class: "yntra-toast-close",
                onclick: move |_| props.onclose.call(()),
                "×"
            }
        }
    }
}
