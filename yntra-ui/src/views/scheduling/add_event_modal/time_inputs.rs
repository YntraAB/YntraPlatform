use dioxus::prelude::*;
use yntra_core::WorkspaceTemplateType;
use crate::components;
use crate::locales::t;

#[derive(Props, Clone, PartialEq)]
pub struct TimeInputsProps {
    pub template: WorkspaceTemplateType,
    pub start_date: Signal<String>,
    pub start_time: Signal<String>,
    pub end_date: Signal<String>,
    pub end_time: Signal<String>,
    pub show_overlap: Signal<bool>,
    pub show_breaks: Signal<bool>,
    pub waiting_from: Signal<String>,
    pub waiting_to: Signal<String>,
    pub active1_from: Signal<String>,
    pub active1_to: Signal<String>,
    pub active2_from: Signal<String>,
    pub active2_to: Signal<String>,
    pub active3_from: Signal<String>,
    pub active3_to: Signal<String>,
    pub break_from: Signal<String>,
    pub break_to: Signal<String>,
    pub is_break_paid: Signal<bool>,
    pub locale: String,
}

#[component]
pub fn TimeInputs(props: TimeInputsProps) -> Element {
    let mut start_date = props.start_date;
    let mut start_time = props.start_time;
    let mut end_date = props.end_date;
    let mut end_time = props.end_time;
    let show_overlap = props.show_overlap;
    let show_breaks = props.show_breaks;
    let mut waiting_from = props.waiting_from;
    let mut waiting_to = props.waiting_to;
    let active1_from = props.active1_from;
    let active1_to = props.active1_to;
    let active2_from = props.active2_from;
    let active2_to = props.active2_to;
    let active3_from = props.active3_from;
    let active3_to = props.active3_to;
    let mut break_from = props.break_from;
    let mut break_to = props.break_to;
    let mut is_break_paid = props.is_break_paid;

    rsx! {
        // Date & Time Grid
        if props.template == WorkspaceTemplateType::Care {
            div { class: "grid grid-cols-2 gap-x-4 gap-y-3 rounded-xl border border-border/50 bg-muted/30 p-4",
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                        "{t(\"common-start-date\", &props.locale)}"
                    }
                    input {
                        r#type: "date",
                        class: "w-full rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{start_date}",
                        oninput: move |e| {
                            start_date.set(e.value());
                            if *start_date.read() == *end_date.read() || end_date.read().is_empty() {
                                end_date.set(e.value());
                            }
                        }
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                        "{t(\"common-start_time\", &props.locale)}"
                    }
                    input {
                        r#type: "time",
                        class: "w-full rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{start_time}",
                        oninput: move |e| start_time.set(e.value())
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                        "{t(\"common-end-date\", &props.locale)}"
                    }
                    input {
                        r#type: "date",
                        class: "w-full rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{end_date}",
                        oninput: move |e| end_date.set(e.value())
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                        "{t(\"common-end_time\", &props.locale)}"
                    }
                    input {
                        r#type: "time",
                        class: "w-full rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{end_time}",
                        oninput: move |e| end_time.set(e.value())
                    }
                }
            }
        } else {
            div { class: "grid grid-cols-3 gap-x-4 gap-y-3 rounded-xl border border-border/50 bg-muted/30 p-4",
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                        "{t(\"common-date\", &props.locale)}"
                    }
                    input {
                        r#type: "date",
                        class: "w-full rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{start_date}",
                        oninput: move |e| {
                            start_date.set(e.value().clone());
                            end_date.set(e.value());
                        }
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                        "{t(\"common-start_time\", &props.locale)}"
                    }
                    input {
                        r#type: "time",
                        class: "w-full rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{start_time}",
                        oninput: move |e| start_time.set(e.value())
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                        "{t(\"common-end_time\", &props.locale)}"
                    }
                    input {
                        r#type: "time",
                        class: "w-full rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{end_time}",
                        oninput: move |e| end_time.set(e.value())
                    }
                }
            }
        }

        // Collapsible 1: Overlap, waiting & active times
        if props.template == WorkspaceTemplateType::Care {
            div { class: "flex flex-col gap-2",
                button {
                    r#type: "button",
                    class: "flex w-full items-center justify-between rounded-lg border border-border/50 bg-muted/20 px-4 py-2.5 text-sm font-medium transition-all hover:bg-muted/30 cursor-pointer",
                    onclick: move |_| {
                        let curr = *show_overlap.read();
                        let mut s = show_overlap;
                        s.set(!curr);
                    },
                    span { "{t(\"scheduler-overlap-waiting-active\", &props.locale)}" }
                    components::LucideIcon { name: "chevron-down", class: "h-4 w-4 text-muted-foreground" }
                }
                if *show_overlap.read() {
                    div { class: "mt-1 space-y-4 rounded-lg border border-border/30 bg-muted/10 p-4 animate-in fade-in duration-200",
                        // Waiting Time
                        div { class: "flex flex-col gap-2",
                            label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                                "{t(\"scheduler-waiting-time\", &props.locale)}"
                            }
                            div { class: "grid grid-cols-2 gap-3",
                                div { class: "flex items-center gap-2",
                                    span { class: "text-[10px] text-muted-foreground", "{t(\"common-from\", &props.locale)}" }
                                    input {
                                        r#type: "time",
                                        class: "w-full rounded border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary",
                                        value: "{waiting_from}",
                                        oninput: move |e| waiting_from.set(e.value())
                                    }
                                }
                                div { class: "flex items-center gap-2",
                                    span { class: "text-[10px] text-muted-foreground", "{t(\"common-to\", &props.locale)}" }
                                    input {
                                        r#type: "time",
                                        class: "w-full rounded border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary",
                                        value: "{waiting_to}",
                                        oninput: move |e| waiting_to.set(e.value())
                                    }
                                }
                            }
                        }
                        // Active times 1, 2, 3
                        for (idx, (from_sig, to_sig)) in [
                            (active1_from, active1_to),
                            (active2_from, active2_to),
                            (active3_from, active3_to)
                        ].into_iter().enumerate() {
                            div { class: "flex flex-col gap-2",
                                label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                                    "{t(\"scheduler-active-time\", &props.locale)} {idx + 1}"
                                }
                                div { class: "grid grid-cols-2 gap-3",
                                    div { class: "flex items-center gap-2",
                                        span { class: "text-[10px] text-muted-foreground", "{t(\"common-from\", &props.locale)}" }
                                        input {
                                            r#type: "time",
                                            class: "w-full rounded border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary",
                                            value: "{from_sig}",
                                            oninput: move |e| {
                                                let mut s = from_sig;
                                                s.set(e.value());
                                            }
                                        }
                                    }
                                    div { class: "flex items-center gap-2",
                                        span { class: "text-[10px] text-muted-foreground", "{t(\"common-to\", &props.locale)}" }
                                        input {
                                            r#type: "time",
                                            class: "w-full rounded border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary",
                                            value: "{to_sig}",
                                            oninput: move |e| {
                                                let mut s = to_sig;
                                                s.set(e.value());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Collapsible 2: Breaks
        div { class: "flex flex-col gap-2",
            button {
                r#type: "button",
                class: "flex w-full items-center justify-between rounded-lg border border-border/50 bg-muted/20 px-4 py-2.5 text-sm font-medium transition-all hover:bg-muted/30 cursor-pointer",
                onclick: move |_| {
                    let curr = *show_breaks.read();
                    let mut s = show_breaks;
                    s.set(!curr);
                },
                span { "{t(\"scheduler-breaks\", &props.locale)}" }
                components::LucideIcon { name: "chevron-down", class: "h-4 w-4 text-muted-foreground" }
            }
            if *show_breaks.read() {
                div { class: "mt-1 space-y-4 rounded-lg border border-border/30 bg-muted/10 p-4 animate-in fade-in duration-200",
                    div { class: "flex flex-col gap-2",
                        label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-70",
                            "{t(\"scheduler-break\", &props.locale)}"
                        }
                        div { class: "grid grid-cols-2 gap-3",
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-muted-foreground", "{t(\"common-from\", &props.locale)}" }
                                input {
                                    r#type: "time",
                                    class: "w-full rounded border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary",
                                    value: "{break_from}",
                                    oninput: move |e| break_from.set(e.value())
                                }
                            }
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-muted-foreground", "{t(\"common-to\", &props.locale)}" }
                                input {
                                    r#type: "time",
                                    class: "w-full rounded border border-border bg-background px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary",
                                    value: "{break_to}",
                                    oninput: move |e| break_to.set(e.value())
                                }
                            }
                        }
                    }
                    div { class: "flex items-center gap-2 pt-1",
                        input {
                            r#type: "checkbox",
                            id: "break-paid-chk",
                            class: "cursor-pointer rounded border-border bg-muted/50",
                            checked: *is_break_paid.read(),
                            onchange: move |e| is_break_paid.set(e.value().parse::<bool>().unwrap_or(false))
                        }
                        label {
                            r#for: "break-paid-chk",
                            class: "cursor-pointer text-xs font-medium text-muted-foreground select-none",
                            "{t(\"scheduler-paid\", &props.locale)}"
                        }
                    }
                }
            }
        }
    }
}
