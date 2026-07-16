use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Dialog, Input, LucideIcon, SuggestionInput};
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{
    checkout_book, create_school_invoice, get_assignments, get_library_books,
    get_library_lending_logs, get_school_invoices, get_student_profiles, get_workspace_courses,
    get_users, record_school_payment, return_book, save_assignment, save_attendance_record, save_course,
    link_parent_to_student, get_student_parents, get_student_health_records, save_student_health_record,
    get_health_incidents, save_health_incident, save_student_profile,
    get_course_term_grades, save_term_grade, publish_report_card, get_report_cards,
    get_student_submissions, save_submission, get_timetable_slots, save_timetable_slot,
    Assignment, Course, SchoolInvoice, HealthRecord, HealthIncident, StudentProfile, TermGrade, ReportCard,
    Submission, TimetableSlot,
};

use super::SchoolViewProps;

fn format_amount(amount: f64, locale: &str) -> String {
    match locale {
        "sv" => format!("{:.2} kr", amount),
        "no" => format!("{:.2} kr", amount),
        "da" => format!("{:.2} kr", amount),
        "fi" => format!("€{:.2}", amount),
        _ => format!("${:.2}", amount),
    }
}

fn get_currency_label(locale: &str) -> String {
    match locale {
        "sv" => "Amount (SEK)".to_string(),
        "no" => "Amount (NOK)".to_string(),
        "da" => "Amount (DKK)".to_string(),
        "fi" => "Amount (EUR)".to_string(),
        _ => "Amount (USD)".to_string(),
    }
}

#[component]
pub fn FinanceView(props: SchoolViewProps) -> Element {
    let mut db_trigger = props.db_trigger;
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();
    let locale = props.locale.clone();

    // Local states
    let mut show_invoice_modal = use_signal(|| false);
    let mut invoice_student_id = use_signal(String::new);
    let mut invoice_title = use_signal(String::new);
    let mut invoice_amount = use_signal(|| 1500.0);
    let invoice_due = use_signal(|| "2026-08-01".to_string());

    let mut context_menu_open = use_signal(|| false);
    let mut context_menu_pos = use_signal(|| (0, 0));
    let mut context_menu_invoice = use_signal(|| Option::<SchoolInvoice>::None);

    let db_trig_val = *db_trigger.read();
    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let invoices_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { get_school_invoices(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let students_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move { get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    let invoices = invoices_res.read().clone().unwrap_or_default();
    let students = students_res.read().clone().unwrap_or_default();

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "flex items-center justify-between border-b border-border pb-4",
                div {
                    h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                        LucideIcon { name: "credit-card", class: "h-6 w-6 text-primary" }
                        "Tuition & Billing"
                    }
                    p { class: "text-xs text-muted-foreground m-0 mt-1", "Track outstanding student tuition fees, create custom bills, and record payments." }
                }
                Button {
                    class: "flex items-center gap-1.5 text-xs h-9 px-4 rounded-xl",
                    onclick: move |_| show_invoice_modal.set(true),
                    LucideIcon { name: "plus", class: "h-4 w-4" }
                    "Issue Invoice"
                }
            }

            // Invoices table
            Card { class: "border-border shadow-sm",
                CardHeader {
                    CardTitle { "Billing Ledger" }
                    CardDescription { "Student tuition statements and payment statuses" }
                }
                CardContent {
                    if invoices.is_empty() {
                        div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl",
                            "No active invoices recorded. Issue a new invoice to get started."
                        }
                    } else {
                        div { class: "overflow-x-auto",
                            table { class: "w-full border-collapse text-sm text-left",
                                thead { class: "bg-muted/40 text-muted-foreground text-xs uppercase font-semibold border-b border-border",
                                    tr {
                                        th { class: "p-3.5", "Student" }
                                        th { class: "p-3.5", "Description" }
                                        th { class: "p-3.5", "Amount" }
                                        th { class: "p-3.5", "Due Date" }
                                        th { class: "p-3.5", "Status" }
                                        th { class: "p-3.5 text-right", "Actions" }
                                    }
                                }
                                tbody { class: "divide-y divide-border",
                                    for inv in invoices.iter() {
                                        {
                                            let inv_c = inv.clone();
                                            let inv_context = inv.clone();
                                            let inv_id = inv.id.clone();
                                            let is_unpaid = inv.status == "unpaid";
                                            let student_name = students.iter()
                                                .find(|s| s.id == inv.student_id)
                                                .map(|s| format!("{} {}", s.first_name, s.last_name))
                                                .unwrap_or_else(|| "Unknown Student".to_string());
                                            let uid = user_id.clone();
                                            let ws = ws_id.clone();
                                            rsx! {
                                                tr {
                                                    key: "{inv_id}",
                                                    class: "hover:bg-muted/20 transition-colors",
                                                    oncontextmenu: move |evt| {
                                                        evt.prevent_default();
                                                        let coords = evt.client_coordinates();
                                                        context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                        context_menu_invoice.set(Some(inv_context.clone()));
                                                        context_menu_open.set(true);
                                                    },
                                                    td { class: "p-3.5 font-medium text-foreground", "{student_name}" }
                                                    td { class: "p-3.5 text-muted-foreground", "{inv.title}" }
                                                    td { class: "p-3.5 text-foreground font-semibold", "{format_amount(inv.amount, &locale)}" }
                                                    td { class: "p-3.5 text-muted-foreground", "{inv.due_date}" }
                                                    td { class: "p-3.5",
                                                        span { class: format!(
                                                            "text-[10px] px-2 py-0.5 rounded-full font-bold uppercase {}",
                                                            if is_unpaid { "bg-red-500/10 text-red-600 border border-red-500/10" } else { "bg-green-500/10 text-green-600 border border-green-500/10" }
                                                        ),
                                                            "{inv.status}"
                                                        }
                                                    }
                                                    td { class: "p-3.5 text-right",
                                                        if is_unpaid {
                                                            Button {
                                                                class: "text-xs h-7 px-3 rounded-lg border border-primary text-primary hover:bg-primary/5 font-semibold",
                                                                onclick: move |_| {
                                                                    let uid_c = uid.clone();
                                                                    let ws_c = ws.clone();
                                                                    let inv_id_c = inv_id.clone();
                                                                    spawn(async move {
                                                                        let _ = record_school_payment(uid_c, ws_c, inv_id_c, "Cash".to_string()).await;
                                                                    });
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                },
                                                                "Record Cash Payment"
                                                            }
                                                        } else {
                                                            span { class: "text-xs text-muted-foreground", "Paid at: {inv.paid_at.clone().unwrap_or_default()}" }
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

            // Custom Invoice Modal
            if *show_invoice_modal.read() {
                Dialog {
                    open: *show_invoice_modal.read(),
                    title: "Issue Custom Tuition Fee Invoice",
                    onclose: move |_| show_invoice_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Select Student" }
                            select {
                                class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none",
                                value: invoice_student_id.read().clone(),
                                onchange: move |evt: FormEvent| invoice_student_id.set(evt.value()),
                                option { value: "", "Choose student..." }
                                for s in students.iter() {
                                    option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                                }
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Invoice Description" }
                            Input {
                                placeholder: "e.g., Semester Tuition Fee - Fall 2026",
                                value: invoice_title.read().clone(),
                                oninput: move |evt: FormEvent| invoice_title.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "{get_currency_label(&locale)}" }
                            Input {
                                placeholder: "1500.00",
                                value: format!("{}", *invoice_amount.read()),
                                oninput: move |evt: FormEvent| {
                                    if let Ok(v) = evt.value().parse::<f64>() {
                                        invoice_amount.set(v);
                                    }
                                },
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_invoice_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                onclick: {
                                    let uid = user_id.clone();
                                    let ws = ws_id.clone();
                                    move |_| {
                                        let inv = SchoolInvoice {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            workspace_id: ws.clone(),
                                            student_id: invoice_student_id.read().clone(),
                                            title: invoice_title.read().clone(),
                                            amount: *invoice_amount.read(),
                                            due_date: invoice_due.read().clone(),
                                            status: "unpaid".to_string(),
                                            paid_at: None,
                                            updated_at: 0,
                                        };
                                        let uid_c = uid.clone();
                                        spawn(async move {
                                            let _ = create_school_invoice(uid_c, inv).await;
                                        });
                                        invoice_title.set(String::new());
                                        show_invoice_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                },
                                "Issue Invoice"
                            }
                        }
                    }
                }
            }

            // Context Menu Overlay
            if let Some(invoice) = context_menu_invoice.read().clone() {
                {
                    let inv = invoice.clone();
                    let inv_pay = invoice.clone();
                    let is_unpaid = inv.status == "unpaid";

                    let student_name = students.iter()
                        .find(|s| s.id == inv.student_id)
                        .map(|s| format!("{} {}", s.first_name, s.last_name))
                        .unwrap_or_else(|| "Unknown Student".to_string());

                    let copy_text = format!(
                        "Invoice: {}\nStudent: {}\nAmount: {}\nDue Date: {}\nStatus: {}",
                        inv.title,
                        student_name,
                        format_amount(inv.amount, &locale),
                        inv.due_date,
                        inv.status
                    );

                    rsx! {
                        crate::components::ContextMenu {
                            open: *context_menu_open.read(),
                            x: context_menu_pos.read().0,
                            y: context_menu_pos.read().1,
                            onclose: move |_| context_menu_open.set(false),

                            if is_unpaid {
                                button {
                                    class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                    onclick: move |_| {
                                        let uid_c = user_id.clone();
                                        let ws_c = ws_id.clone();
                                        let inv_id_c = inv_pay.id.clone();
                                        spawn(async move {
                                            let _ = record_school_payment(uid_c, ws_c, inv_id_c, "Cash".to_string()).await;
                                        });
                                        context_menu_open.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    },
                                    crate::components::LucideIcon { name: "credit-card", size: "14" }
                                    "Record Payment"
                                }
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    let js = format!("navigator.clipboard.writeText({:?});", copy_text);
                                    let _ = dioxus::document::eval(&js);
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "copy", size: "14" }
                                "Copy Invoice Details"
                            }
                        }
                    }
                }
            }
        }
    }
}

