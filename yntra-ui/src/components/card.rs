use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct CardProps {
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    #[props(optional)]
    pub onclick: Option<EventHandler<MouseEvent>>,
    pub children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct CardHeaderProps {
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    pub children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct CardTitleProps {
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    pub children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct CardDescriptionProps {
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    pub children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct CardContentProps {
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    pub children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct CardFooterProps {
    #[props(default = String::new())]
    pub class: String,
    #[props(default = String::new())]
    pub style: String,
    pub children: Element,
}

#[component]
pub fn Card(props: CardProps) -> Element {
    rsx! {
        div {
            class: "rounded-xl border bg-card text-card-foreground shadow {props.class}",
            style: "{props.style}",
            onclick: move |evt| {
                if let Some(ref handler) = props.onclick {
                    handler.call(evt);
                }
            },
            {props.children}
        }
    }
}

#[component]
pub fn CardHeader(props: CardHeaderProps) -> Element {
    rsx! {
        div {
            class: "flex flex-col space-y-1.5 p-6 {props.class}",
            style: "{props.style}",
            {props.children}
        }
    }
}

#[component]
pub fn CardTitle(props: CardTitleProps) -> Element {
    rsx! {
        h3 {
            class: "font-semibold leading-none tracking-tight {props.class}",
            style: "{props.style}",
            {props.children}
        }
    }
}

#[component]
pub fn CardDescription(props: CardDescriptionProps) -> Element {
    rsx! {
        p {
            class: "text-sm text-muted-foreground {props.class}",
            style: "{props.style}",
            {props.children}
        }
    }
}

#[component]
pub fn CardContent(props: CardContentProps) -> Element {
    rsx! {
        div {
            class: "p-6 pt-0 {props.class}",
            style: "{props.style}",
            {props.children}
        }
    }
}

#[component]
pub fn CardFooter(props: CardFooterProps) -> Element {
    rsx! {
        div {
            class: "flex items-center p-6 pt-0 {props.class}",
            style: "{props.style}",
            {props.children}
        }
    }
}

