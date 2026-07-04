use dioxus::prelude::*;
use dioxus_primitives::dialog::{self, DialogDescriptionProps, DialogTitleProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[css_module("/src/components/dialog/style.css")]
struct Styles;

#[derive(Props, Clone, PartialEq)]
pub struct DialogProps {
    pub open: bool,
    pub onclose: EventHandler<()>,
    pub title: String,
    pub children: Element,
    #[props(optional)]
    pub max_width: Option<String>,
}

#[component]
pub fn Dialog(props: DialogProps) -> Element {
    let onclose = props.onclose;
    let on_open_change = Callback::new(move |open: bool| {
        if !open {
            onclose.call(());
        }
    });

    let max_width_style = if let Some(width) = &props.max_width {
        format!("max-width: {};", width)
    } else {
        "".to_string()
    };

    rsx! {
        dialog::DialogRoot {
            class: Styles::dx_dialog_backdrop,
            open: props.open,
            on_open_change: on_open_change,
            dialog::DialogContent {
                class: Some(Styles::dx_dialog.to_string()),
                style: "{max_width_style}",
                button {
                    class: Styles::dx_dialog_close,
                    onclick: move |_| onclose.call(()),
                    "×"
                }
                h2 { class: Styles::dx_dialog_title, "{props.title}" }
                div { class: "yntra-dialog-body", {props.children} }
            }
        }
    }
}

#[component]
pub fn DialogTitle(props: DialogTitleProps) -> Element {
    let base = attributes!(h2 {
        class: Styles::dx_dialog_title,
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dialog::DialogTitle {
            id: props.id,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn DialogDescription(props: DialogDescriptionProps) -> Element {
    let base = attributes!(p {
        class: Styles::dx_dialog_description,
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dialog::DialogDescription {
            id: props.id,
            attributes: merged,
            {props.children}
        }
    }
}
