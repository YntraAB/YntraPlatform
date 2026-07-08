use dioxus::prelude::*;
use crate::components;
use crate::locales;

#[derive(Props, Clone, PartialEq)]
pub struct IssueInvoiceCardProps {
    pub workspace_id: String,
    pub student_id: String,
    pub active_user_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn IssueInvoiceCard(props: IssueInvoiceCardProps) -> Element {
    let mut db_trigger = props.db_trigger;
    let mut invoice_title = use_signal(String::new);
    let mut invoice_amount = use_signal(|| 0.0);
    let mut invoice_due = use_signal(String::new);

    rsx! {
        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-3",
            h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                components::LucideIcon { name: "plus", size: "16", class: "text-primary" }
                {locales::t("school-billing-issue", &props.locale)}
            }

            // Form Title
            div { class: "flex flex-col gap-1",
                label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Invoice Title" }
                input {
                    r#type: "text",
                    placeholder: "e.g. Fall Tuition, Field Trip",
                    class: "yntra-input text-xs w-full",
                    value: "{invoice_title}",
                    oninput: move |e| invoice_title.set(e.value()),
                }
            }

            // Form Amount
            div { class: "flex flex-col gap-1",
                label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Amount ($)" }
                input {
                    r#type: "number",
                    class: "yntra-input text-xs w-full",
                    value: "{invoice_amount}",
                    oninput: move |e| invoice_amount.set(e.value().parse::<f64>().unwrap_or(0.0)),
                }
            }

            // Form Due Date
            div { class: "flex flex-col gap-1",
                label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Due Date" }
                input {
                    r#type: "date",
                    class: "yntra-input text-xs w-full",
                    value: "{invoice_due}",
                    oninput: move |e| invoice_due.set(e.value()),
                }
            }

            button {
                class: "yntra-btn text-xs font-bold py-2 w-full mt-2 flex items-center justify-center gap-1.5 shadow-sm",
                onclick: {
                    let ws = props.workspace_id.clone();
                    let s_id = props.student_id.clone();
                    let uid = props.active_user_id.clone();
                    move |_| {
                        let ws_c = ws.clone();
                        let s_id_c = s_id.clone();
                        let title = invoice_title.read().clone();
                        let amount = *invoice_amount.read();
                        let due = invoice_due.read().clone();
                        let uid_c = uid.clone();
                        
                        spawn(async move {
                            if yntra_core::save_school_invoice(uid_c, None, ws_c, s_id_c, title, amount, due, "unpaid".to_string(), None).await.is_ok() {
                                invoice_title.set(String::new());
                                invoice_amount.set(0.0);
                                invoice_due.set(String::new());
                                let current = *db_trigger.read();
                                db_trigger.set(current + 1);
                            }
                        });
                    }
                },
                components::LucideIcon { name: "plus", size: "14" }
                "Issue Invoice"
            }
        }
    }
}
