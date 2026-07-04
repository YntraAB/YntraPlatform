use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ContextMenuProps {
    pub open: bool,
    pub x: i32,
    pub y: i32,
    pub onclose: EventHandler<()>,
    pub children: Element,
}

#[component]
pub fn ContextMenu(props: ContextMenuProps) -> Element {
    if !props.open {
        return rsx! {};
    }

    // Capture outside clicks to close context menu
    use_effect(move || {
        spawn(async move {
            // Simply close on any click outside after short duration
            crate::utils::sleep_ms(50).await;
        });
    });

    let position_style = format!("top: {}px; left: {}px;", props.y, props.x);

    rsx! {
        div {
            class: "yntra-context-menu-overlay",
            onclick: move |_| props.onclose.call(()),
            style {
                r#"
                .yntra-context-menu-overlay {{
                    position: fixed;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    z-index: 1500;
                    background: transparent;
                }}
                .yntra-context-menu {{
                    position: absolute;
                    background: hsl(220.9, 39.3%, 11%);
                    border: 1px solid rgba(255, 255, 255, 0.08);
                    border-radius: 8px;
                    min-width: 160px;
                    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.4);
                    padding: 0.25rem;
                    display: flex;
                    flex-direction: column;
                    z-index: 1600;
                }}
                "#
            }
            div {
                class: "yntra-context-menu",
                style: "{position_style}",
                onclick: move |evt| evt.stop_propagation(),
                {props.children}
            }
        }
    }
}
