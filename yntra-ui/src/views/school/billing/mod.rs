use crate::components;
use crate::locales;
use dioxus::prelude::*;
use yntra_core::StudentProfile;

pub mod receipt_modal;
pub mod issue_form;
use receipt_modal::ReceiptModalDialog;
use issue_form::IssueInvoiceCard;

#[derive(Props, Clone)]
pub struct BillingDashboardProps {
    pub active_user_id: String,
    pub students: Vec<StudentProfile>,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for BillingDashboardProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn BillingDashboard(props: BillingDashboardProps) -> Element {
    let active_user_id = props.active_user_id.clone();
    let students = props.students.clone();
    let workspace_id = props.workspace_id.clone();
    let mut db_trigger = props.db_trigger;
    let locale = props.locale.clone();
    let db_trig = *db_trigger.read();

    let mut selected_student_id = use_signal(|| students.first().map(|s| s.id.clone()));
    let selected_student = students.iter().find(|s| Some(s.id.clone()) == *selected_student_id.read()).cloned();

    let mut selected_invoice_for_receipt = use_signal(|| Option::<yntra_core::SchoolInvoice>::None);

    // Fetch invoices for selected student
    let s_id = selected_student_id.read().clone().unwrap_or_default();
    let uid = active_user_id.clone();
    let invoices_res = use_resource(move || {
        let _ = db_trig;
        let id = s_id.clone();
        let u = uid.clone();
        async move {
            yntra_core::get_school_invoices(u, id).await.unwrap_or_default()
        }
    });
    let invoices = invoices_res.read().clone().unwrap_or_default();

    // Manual payment form state
    let mut recording_payment_invoice_id = use_signal(|| Option::<String>::None);
    let mut payment_method = use_signal(|| "bank_transfer".to_string());

    // Calculate outstanding balance
    let outstanding_balance: f64 = invoices.iter()
        .filter(|i| i.status == "unpaid")
        .map(|i| i.amount)
        .sum();

    rsx! {
        div { class: "flex flex-col gap-6",
            // Main Grid
            div { class: "grid gap-6 lg:grid-cols-4 items-start",
                // Left Column: Student roster list
                div { class: "lg:col-span-1 flex flex-col gap-4 bg-sidebar/20 border border-border/40 p-4 rounded-xl shadow-sm",
                    h3 { class: "text-xs font-black uppercase text-muted-foreground tracking-wider m-0 px-1 pb-2 border-b border-border/30",
                        {locales::t("school-report-roster", &locale)}
                    }
                    div { class: "flex flex-col gap-1.5 max-h-[500px] overflow-y-auto",
                        for s in students.iter() {
                            {
                                let s_id = s.id.clone();
                                let s_name = format!("{} {}", s.first_name, s.last_name);
                                let s_grade = s.grade_level.clone();
                                let is_selected = Some(s_id.clone()) == *selected_student_id.read();
                                let bg_cls = if is_selected { "bg-primary/10 border-primary text-primary" } else { "bg-transparent border-transparent text-muted-foreground hover:bg-muted hover:text-foreground" };
                                rsx! {
                                    button {
                                        key: "{s_id}",
                                        class: "w-full text-left p-3 rounded-xl border transition-all flex justify-between items-center {bg_cls}",
                                        onclick: move |_| {
                                            selected_student_id.set(Some(s_id.clone()));
                                            recording_payment_invoice_id.set(None);
                                        },
                                        div { class: "flex flex-col gap-0.5",
                                            span { class: "text-xs font-black", "{s_name}" }
                                            span { class: "text-[10px] opacity-75 font-bold", "Grade: {s_grade}" }
                                        }
                                        components::LucideIcon { name: "chevron-right", size: "14" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right columns: Selected Student billing information
                div { class: "lg:col-span-3 flex flex-col gap-6",
                    if let Some(ref student) = selected_student {
                        div { class: "flex flex-col gap-6",
                            // Outstanding balance display
                            div { class: "flex justify-between items-center p-5 border border-border/40 bg-sidebar/10 rounded-xl flex-wrap gap-4",
                                div { class: "flex items-center gap-3",
                                    div { class: "p-2 rounded-lg bg-primary/10 text-primary",
                                        components::LucideIcon { name: "wallet", size: "20" }
                                    }
                                    div { class: "flex flex-col gap-0.5",
                                        h3 { class: "text-base font-black text-foreground m-0", "{student.first_name} {student.last_name}" }
                                        span { class: "text-xs text-muted-foreground font-semibold", "Workspace Student Profile" }
                                    }
                                }
                                
                                div { class: "flex flex-col items-end gap-0.5",
                                    span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", {locales::t("school-billing-outstanding", &locale)} }
                                    span { class: "text-xl font-black text-foreground", "${outstanding_balance:.2}" }
                                }
                            }

                            // Sub-grids: Invoice form & Invoice list
                            div { class: "grid gap-6 md:grid-cols-3 items-start",
                                // Issue Invoice form
                                div { class: "md:col-span-1 flex flex-col gap-4",
                                    IssueInvoiceCard {
                                        workspace_id: workspace_id.clone(),
                                        student_id: student.id.clone(),
                                        active_user_id: active_user_id.clone(),
                                        db_trigger,
                                        locale: locale.clone(),
                                    }
                                }

                                // Invoice List
                                div { class: "md:col-span-2 flex flex-col gap-4",
                                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                        h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                            components::LucideIcon { name: "time", size: "16", class: "text-primary" }
                                            {locales::t("school-billing-invoices", &locale)}
                                        }

                                        if invoices.is_empty() {
                                            p { class: "text-xs text-muted-foreground italic text-center p-8 m-0", "No invoices registered." }
                                        } else {
                                            div { class: "flex flex-col gap-3.5",
                                                for invoice in invoices.iter() {
                                                    {
                                                        let inv_id = invoice.id.clone();
                                                        let inv_title = invoice.title.clone();
                                                        let is_unpaid = invoice.status == "unpaid";
                                                        let is_recording_payment = Some(inv_id.clone()) == *recording_payment_invoice_id.read();
                                                        
                                                        rsx! {
                                                            div { key: "{inv_id}", class: "border border-border/30 p-4 rounded-xl bg-sidebar/40 flex flex-col gap-3",
                                                                div { class: "flex justify-between items-center",
                                                                    div { class: "flex flex-col gap-0.5",
                                                                        span { class: "text-sm font-black text-foreground", "{inv_title}" }
                                                                        span { class: "text-[10px] text-muted-foreground font-semibold", "Due: {invoice.due_date}" }
                                                                    }
                                                                    
                                                                    div { class: "flex items-center gap-2.5",
                                                                        span { class: "text-sm font-black text-foreground", "${invoice.amount:.2}" }
                                                                        match invoice.status.as_str() {
                                                                            "paid" => rsx! { 
                                                                                div { class: "flex items-center gap-1.5",
                                                                                    span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Paid" }
                                                                                    button {
                                                                                        class: "px-2 py-0.5 rounded text-[9px] font-bold bg-sidebar hover:bg-muted text-muted-foreground border border-border/40 transition-all flex items-center gap-1",
                                                                                        onclick: {
                                                                                            let inv = invoice.clone();
                                                                                            move |_| selected_invoice_for_receipt.set(Some(inv.clone()))
                                                                                        },
                                                                                        components::LucideIcon { name: "award", size: "10" }
                                                                                        "Receipt"
                                                                                    }
                                                                                }
                                                                            },
                                                                            _ => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-amber-500/10 text-amber-400 border border-amber-500/15 shadow-sm animate-pulse", "Unpaid" } }
                                                                        }
                                                                    }
                                                                }
                                                                
                                                                if is_unpaid {
                                                                    if is_recording_payment {
                                                                        div { class: "mt-1 pt-2 border-t border-border/30 flex justify-between items-center gap-3",
                                                                            div { class: "flex items-center gap-2 flex-1",
                                                                                label { class: "text-[9px] font-bold text-muted-foreground whitespace-nowrap", "Method:" }
                                                                                select {
                                                                                    class: "yntra-input py-1 text-[10px] bg-sidebar w-full",
                                                                                    value: "{payment_method}",
                                                                                    onchange: move |e| payment_method.set(e.value()),
                                                                                    option { value: "bank_transfer", "Bank Transfer" }
                                                                                    option { value: "cash", "Cash" }
                                                                                }
                                                                            }
                                                                            
                                                                            div { class: "flex gap-1.5",
                                                                                button {
                                                                                    class: "px-2 py-1 text-[9px] font-bold rounded bg-muted hover:bg-muted/80 text-foreground transition-all",
                                                                                    onclick: move |_| recording_payment_invoice_id.set(None),
                                                                                    "Cancel"
                                                                                }
                                                                                button {
                                                                                    class: "px-2 py-1 text-[9px] font-bold rounded bg-emerald-500 text-white hover:bg-emerald-600 transition-all",
                                                                                    onclick: {
                                                                                        let ws = workspace_id.clone();
                                                                                        let i_id = inv_id.clone();
                                                                                        let amount = invoice.amount;
                                                                                        let uid = active_user_id.clone();
                                                                                        move |_| {
                                                                                            let ws_c = ws.clone();
                                                                                            let i_id_c = i_id.clone();
                                                                                            let pm = payment_method.read().clone();
                                                                                            let uid_c = uid.clone();
                                                                                            let now_str = yntra_core::infra::time::get_current_datetime_str();
                                                                                            
                                                                                            spawn(async move {
                                                                                                if yntra_core::record_school_payment(uid_c, ws_c, i_id_c, amount, pm, now_str).await.is_ok() {
                                                                                                    recording_payment_invoice_id.set(None);
                                                                                                    let current = *db_trigger.read();
                                                                                                    db_trigger.set(current + 1);
                                                                                                }
                                                                                            });
                                                                                        }
                                                                                    },
                                                                                    "Confirm Payment"
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        div { class: "mt-1 pt-2 border-t border-border/30 flex justify-end",
                                                                            button {
                                                                                class: "px-2.5 py-1 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-500/20 transition-all flex items-center gap-1.5",
                                                                                onclick: move |_| {
                                                                                    recording_payment_invoice_id.set(Some(inv_id.clone()));
                                                                                },
                                                                                components::LucideIcon { name: "check", size: "12" }
                                                                                "Record Manual Payment"
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
                                    }
                                }
                            }
                        }
                    } else {
                        p { class: "text-xs text-muted-foreground italic text-center p-8 bg-sidebar/10 rounded-xl", "Please select a student to view billing details." }
                    }
                }
            }
            
            ReceiptModalDialog {
                selected_invoice_for_receipt,
            }
        }
    }
}
