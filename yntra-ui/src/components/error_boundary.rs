use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ErrorBoundaryProps {
    pub children: Element,
}

#[component]
pub fn ErrorBoundary(props: ErrorBoundaryProps) -> Element {
    rsx! {
        dioxus::prelude::ErrorBoundary {
            handle_error: |errors: ErrorContext| {
                rsx! {
                    div { class: "flex flex-col items-center justify-center text-center bg-white/[0.01] border border-border rounded-xl",
                    style: "padding:3rem; min-height:400px; margin:2rem 0;",
                        div { class: "mb-6 rounded-full flex items-center justify-center",
                    style: "background:rgba(239, 68, 68, 0.1); color:hsl(0, 84.2%, 60.2%); width:64px; height:64px;",
                            span { class: "font-bold",
                    style: "font-size:2rem;", "!" }
                        }
                        h2 { class: "mb-3 text-2xl font-bold",
                    style: "color:var(--text-main);",
                            "Something went wrong"
                        }
                        p { class: "text-muted-foreground/60 mb-8 text-sm",
                    style: "max-width:400px;",
                            "An unexpected rendering error occurred. Please clear errors or refresh the page."
                        }
                        button {
                            class: "yntra-btn",
                            onclick: move |_| {
                                errors.clear_errors();
                            },
                            "Try Again"
                        }
                    }
                }
            },
            {props.children}
        }
    }
}
