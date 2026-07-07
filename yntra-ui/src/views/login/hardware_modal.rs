use crate::components;
use crate::locales::t;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct HardwareModalProps {
    pub show_hardware_modal: Signal<bool>,
    pub hardware_title: String,
    pub hardware_reader_status: Signal<String>,
    pub hardware_polling_label: String,
    pub hardware_error_msg: Signal<Option<String>>,
    pub region: String,
    pub on_close: EventHandler<()>,
}

impl PartialEq for HardwareModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn HardwareModal(props: HardwareModalProps) -> Element {
    let mut show_hardware_modal = props.show_hardware_modal;
    let hardware_reader_status = props.hardware_reader_status;
    let hardware_error_msg = props.hardware_error_msg;

    rsx! {
        components::Dialog {
            open: *show_hardware_modal.read(),
            title: props.hardware_title.clone(),
            onclose: move |_| {
                show_hardware_modal.set(false);
                props.on_close.call(());
            },
            div { class: "flex flex-col items-center text-center gap-5 w-full text-sm",
                style: "max-width: 320px;",
                h3 { class: "m-0 font-extrabold text-foreground",
                    "{props.hardware_title}"
                }

                // Status and Animation Display
                match hardware_reader_status.read().as_str() {
                    "connecting" => rsx! {
                        div { class: "dxc-spinner", }
                        p { class: "text-muted-foreground m-0 text-sm",
                            "{t(\"login-hw-connecting\", &props.region)}"
                        }
                    },
                    "polling" => rsx! {
                        // Pulsating sonar sensor animation for scanning
                        div { 
                            class: "flex items-center justify-center",
                            style: "position: relative; width: 80px; height: 80px; margin: 0.5rem 0;",
                            div {
                                class: "w-full h-full rounded-full",
                                style: "position: absolute; border: 2px solid var(--focused-border-color); animation: sonar-wave 1.5s infinite ease-out;",
                            }
                            div {
                                class: "rounded-full border border-border flex items-center justify-center",
                                style: "width: 50px; height: 50px; background: var(--primary-color-4);",
                                components::LucideIcon { 
                                    name: "reporting",
                                    class: "h-5 w-5 text-secondary", 
                                }
                            }
                        }
                        p { class: "text-muted-foreground m-0 text-sm",
                            style: "line-height: 1.4;",
                            "{props.hardware_polling_label}"
                        }
                    },
                    "reading" => rsx! {
                        div { class: "dxc-spinner", }
                        p { class: "text-muted-foreground m-0 text-sm",
                            "{t(\"login-hw-reading\", &props.region)}"
                        }
                    },
                    "success" => rsx! {
                        div {
                            class: "rounded-full flex items-center justify-center",
                            style: "width: 64px; height: 64px; background: rgba(16,185,129,0.1); border: 2px solid var(--success); margin: 0.5rem 0; box-shadow: 0 0 15px rgba(16,185,129,0.25);",
                            components::LucideIcon { name: "reporting", class: "h-8 w-8 text-success", }
                        }
                        h4 { class: "m-0 font-extrabold",
                            style: "color:var(--success);", 
                            "{t(\"login-hw-success\", &props.region)}"
                        }
                    },
                    _ => rsx! {
                        div {
                            class: "rounded-full flex items-center justify-center",
                            style: "width: 64px; height: 64px; background: rgba(239,68,68,0.1); border: 2px solid var(--danger); margin: 0.5rem 0; box-shadow: 0 0 15px rgba(239,68,68,0.25);",
                            components::LucideIcon { name: "reporting", class: "h-8 w-8 text-danger", }
                        }
                        if let Some(err) = hardware_error_msg.read().as_ref() {
                            p { class: "text-xs text-center m-0 font-semibold",
                                style: "color: var(--danger);", "{t(err, &props.region)}" }
                        }
                    }
                }

                // Cancel / Back Button
                button {
                    class: "yntra-btn secondary w-full",
                    onclick: move |_| {
                        show_hardware_modal.set(false);
                        props.on_close.call(());
                    },
                    "{t(\"login-hw-btn-cancel\", &props.region)}"
                }
            }
        }
    }
}
