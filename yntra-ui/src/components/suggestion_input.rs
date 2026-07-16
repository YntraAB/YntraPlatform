use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SuggestionInputProps {
    #[props(default = String::new())]
    pub placeholder: String,
    pub value: String,
    pub suggestions: Vec<String>,
    pub onchange: Callback<String>,
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub id: String,
}

#[component]
pub fn SuggestionInput(props: SuggestionInputProps) -> Element {
    let mut show_dropdown = use_signal(|| false);
    let onchange = props.onchange;

    let filtered_suggestions = {
        let val_lower = props.value.to_lowercase();
        props.suggestions
            .iter()
            .filter(|s| s.to_lowercase().contains(&val_lower))
            .cloned()
            .collect::<Vec<String>>()
    };

    rsx! {
        div { class: "relative w-full",
            input {
                class: "yntra-input {props.class}",
                r#type: "text",
                placeholder: "{props.placeholder}",
                value: "{props.value}",
                id: if props.id.is_empty() { None } else { Some(props.id.clone()) },
                oninput: move |evt: FormEvent| {
                    onchange.call(evt.value());
                },
                onfocus: move |_| {
                    show_dropdown.set(true);
                },
                onblur: move |_| {
                    // Small delay to allow click events on dropdown options to fire
                    spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(180)).await;
                        show_dropdown.set(false);
                    });
                },
            }
            if *show_dropdown.read() && !filtered_suggestions.is_empty() {
                div { class: "absolute left-0 right-0 mt-1 z-50 rounded-lg border border-border bg-sidebar py-1.5 shadow-2xl max-h-48 overflow-y-auto duration-200 animate-in fade-in slide-in-from-top-1",
                    for s in filtered_suggestions.iter() {
                        {
                            let s_clone = s.clone();
                            rsx! {
                                button {
                                    class: "flex w-full items-center text-left px-3 py-1.5 text-xs text-foreground transition-colors hover:bg-muted/50 border-0 bg-transparent cursor-pointer",
                                    r#type: "button",
                                    onclick: move |_| {
                                        onchange.call(s_clone.clone());
                                        show_dropdown.set(false);
                                    },
                                    "{s}"
                                }
                            }
                        }
                    }
                }
            }
        }
        style {
            r#"
            :where(.yntra-input) {{
                background: rgba(0, 0, 0, 0.2);
                border: 1px solid var(--border-color);
                color: var(--text-primary);
                padding: 0.65rem 0.85rem;
                border-radius: 8px;
                font-size: 0.9rem;
                outline: none;
                transition: border-color 0.2s;
                width: 100%;
                box-sizing: border-box;
            }}
            :where(.yntra-input):focus {{
                border-color: var(--accent-color);
            }}
            "#
        }
    }
}
