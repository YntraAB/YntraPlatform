#![allow(unused_imports)]
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
    get_parent_students,
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
    let state = use_context::<crate::state::AppState>();
    let mut db_trigger = state.trigger_school_finance;
    let trigger_school_finance = state.trigger_school_finance;
    let trigger_school_directory = state.trigger_school_directory;
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();
    let locale = props.locale.clone();

    // Local states
    let mut show_invoice_modal = use_signal(|| false);
    let mut invoice_student_id = use_signal(String::new);
    let mut invoice_title = use_signal(String::new);
    let mut invoice_amount = use_signal(|| 1500.0);
    let invoice_due = use_signal(|| (chrono::Local::now() + chrono::Duration::days(30)).format("%Y-%m-%d").to_string());

    let mut show_payment_modal = use_signal(|| false);
    let mut selected_pay_invoice_id = use_signal(|| Option::<String>::None);
    let mut selected_pay_invoice_title = use_signal(String::new);
    let mut selected_pay_invoice_amount = use_signal(|| 0.0);
    let mut card_number = use_signal(String::new);
    let mut card_expiry = use_signal(String::new);
    let mut card_cvc = use_signal(String::new);
    let mut payment_processing = use_signal(|| false);
    let mut payment_success = use_signal(|| false);

    let mut context_menu_open = use_signal(|| false);
    let mut context_menu_pos = use_signal(|| (0, 0));
    let mut context_menu_invoice = use_signal(|| Option::<SchoolInvoice>::None);

    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let invoices_res = use_resource(move || {
        let _trig = trigger_school_finance.read();
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { get_school_invoices(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let students_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move { get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone_p = user_id.clone();
    let ws_id_clone_p = ws_id.clone();
    let parent_students_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let uid = user_id_clone_p.clone();
        let ws = ws_id_clone_p.clone();
        async move { get_parent_students(uid.clone(), ws, uid).await.unwrap_or_default() }
    });

    let mut selected_student_id = use_signal(|| "".to_string());

    use_effect(move || {
        let role = state.active_user_role.read().clone();
        if role == "parent" || role == "role-school-parent" {
            let student_list = parent_students_res.read().clone().unwrap_or_default();
            if !student_list.is_empty() && selected_student_id.read().is_empty() {
                selected_student_id.set(student_list[0].id.clone());
            }
        }
    });

    let invoices_raw = invoices_res.read().clone().unwrap_or_default();
    let current_role = state.active_user_role.read().clone();
    let students = if current_role == "parent" || current_role == "role-school-parent" {
        parent_students_res.read().clone().unwrap_or_default()
    } else {
        students_res.read().clone().unwrap_or_default()
    };

    let invoices = if current_role == "parent" || current_role == "role-school-parent" {
        let s_id = selected_student_id.read().clone();
        invoices_raw.into_iter().filter(|inv| inv.student_id == s_id).collect::<Vec<_>>()
    } else if current_role == "student" || current_role == "role-school-student" {
        let student_ids: std::collections::HashSet<String> = students.iter()
            .filter(|s| s.user_id.as_ref() == Some(&user_id))
            .map(|s| s.id.clone())
            .collect();
        invoices_raw.into_iter().filter(|inv| student_ids.contains(&inv.student_id)).collect::<Vec<_>>()
    } else {
        invoices_raw
    };

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "flex items-center justify-between border-b border-border pb-4",
                div {
                    h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                        LucideIcon { name: "credit-card", class: "h-6 w-6 text-primary" }
                        {t("school-parent-billing-title", &locale)}
                    }
                    p { class: "text-xs text-muted-foreground m-0 mt-1", {t("school-parent-billing-desc", &locale)} }
                }
                if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                    Button {
                        class: "flex items-center gap-1.5 text-xs h-9 px-4 rounded-xl",
                        onclick: move |_| show_invoice_modal.set(true),
                        LucideIcon { name: "plus", class: "h-4 w-4" }
                        "Issue Invoice"
                    }
                }
            }

            if current_role == "parent" || current_role == "role-school-parent" {
                Card { class: "p-4 border border-border bg-sidebar rounded-2xl flex flex-col sm:flex-row gap-4 items-center justify-between shadow-sm",
                    div { class: "flex items-center gap-3 w-full sm:w-auto",
                        LucideIcon { name: "user", class: "h-5 w-5 text-primary" }
                        div {
                            h4 { class: "text-sm font-bold text-foreground m-0", {t("school-parent-select-child", &locale)} }
                            p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", {t("school-parent-select-child-desc", &locale)} }
                        }
                    }
                    select {
                        class: "rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 w-full sm:w-60",
                        value: selected_student_id.read().clone(),
                        onchange: move |evt: FormEvent| selected_student_id.set(evt.value()),
                        for s in students.iter() {
                            option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                        }
                    }
                }
            }

            // Invoices table
            Card { class: "border-border shadow-sm",
                CardHeader {
                    CardTitle { {t("school-parent-billing-ledger", &locale)} }
                    CardDescription { {t("school-parent-billing-ledger-desc", &locale)} }
                }
                CardContent {
                    if invoices.is_empty() {
                        div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl",
                            if current_role == "parent" || current_role == "role-school-parent" || current_role == "student" || current_role == "role-school-student" {
                                {t("school-parent-billing-no-invoices-parent", &locale)}
                            } else {
                                {t("school-parent-billing-no-invoices-staff", &locale)}
                            }
                        }
                    } else {
                        div { class: "overflow-x-auto",
                            table { class: "w-full border-collapse text-sm text-left",
                                thead { class: "bg-muted/40 text-muted-foreground text-xs uppercase font-semibold border-b border-border",
                                    tr {
                                        th { class: "p-3.5", {t("school-parent-billing-header-student", &locale)} }
                                        th { class: "p-3.5", {t("school-parent-billing-header-desc", &locale)} }
                                        th { class: "p-3.5", {t("school-parent-billing-header-amount", &locale)} }
                                        th { class: "p-3.5", {t("school-parent-billing-header-due", &locale)} }
                                        th { class: "p-3.5", {t("school-parent-billing-header-status", &locale)} }
                                        th { class: "p-3.5 text-right", {t("school-parent-billing-header-actions", &locale)} }
                                    }
                                }
                                tbody { class: "divide-y divide-border",
                                    for inv in invoices.iter() {
                                        {
                                            let current_role = current_role.clone();
                                            let _inv_c = inv.clone();
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
                                                        if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                                                            evt.prevent_default();
                                                            let coords = evt.client_coordinates();
                                                            context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                            context_menu_invoice.set(Some(inv_context.clone()));
                                                            context_menu_open.set(true);
                                                        }
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
                                                            if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                                                                Button {
                                                                    class: "text-xs h-7 px-3 rounded-lg border border-primary text-primary hover:bg-primary/5 font-semibold",
                                                                     onclick: {
                                                                         let uid_c = uid.clone();
                                                                         let ws_c = ws.clone();
                                                                         let inv_id_c = inv_id.clone();
                                                                         let state = state;
                                                                         move |_| {
                                                                             let role = state.active_user_role.read().clone();
                                                                             let u_id = state.active_user_id.read().clone();
                                                                             let proof = yntra_core::ZkCryptoTrust::new()
                                                                             .generate_role_proof(state.get_passkey_seed(), u_id.clone(), role.clone())
                                                                                 .ok();
                                                                             let u = uid_c.clone();
                                                                             let w = ws_c.clone();
                                                                             let i = inv_id_c.clone();
                                                                             spawn(async move {
                                                                                 let _ = record_school_payment(u, w, i, "Cash".to_string(), proof).await;
                                                                             });
                                                                             let current = *db_trigger.read();
                                                                             db_trigger.set(current + 1);
                                                                         }
                                                                     },
                                                                    "Record Cash Payment"
                                                                }
                                                            } else {
                                                                Button {
                                                                    class: "text-xs h-7 px-3 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 font-semibold border-0 cursor-pointer",
                                                                    onclick: {
                                                                        let inv_id_c = inv_id.clone();
                                                                        let title_c = inv.title.clone();
                                                                        let amount_c = inv.amount;
                                                                        move |_| {
                                                                            selected_pay_invoice_id.set(Some(inv_id_c.clone()));
                                                                            selected_pay_invoice_title.set(title_c.clone());
                                                                            selected_pay_invoice_amount.set(amount_c);
                                                                            payment_success.set(false);
                                                                            payment_processing.set(false);
                                                                            card_number.set(String::new());
                                                                            card_expiry.set(String::new());
                                                                            card_cvc.set(String::new());
                                                                            show_payment_modal.set(true);
                                                                        }
                                                                    },
                                                                    {t("school-finance-pay-online", &locale)}
                                                                }
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
                                     let state = state;
                                     move |_| {
                                          let role = state.active_user_role.read().clone();
                                          let u_id = state.active_user_id.read().clone();
                                          let proof = yntra_core::ZkCryptoTrust::new()
                                              .generate_role_proof(state.get_passkey_seed(), u_id.clone(), role.clone())
                                              .ok();
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
                                             let _ = create_school_invoice(uid_c, inv, proof).await;
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
                                     onclick: {
                                         let uid_c = user_id.clone();
                                         let ws_c = ws_id.clone();
                                         let inv_pay_id = inv_pay.id.clone();
                                         let state = state;
                                         move |_| {
                                             let role = state.active_user_role.read().clone();
                                             let u_id = state.active_user_id.read().clone();
                                             let proof = yntra_core::ZkCryptoTrust::new()
                                                .generate_role_proof(state.get_passkey_seed(), u_id.clone(), role.clone())
                                                 .ok();
                                             let u = uid_c.clone();
                                             let w = ws_c.clone();
                                             let inv_id_c = inv_pay_id.clone();
                                             spawn(async move {
                                                 let _ = record_school_payment(u, w, inv_id_c, "Cash".to_string(), proof).await;
                                             });
                                             context_menu_open.set(false);
                                             let current = *db_trigger.read();
                                             db_trigger.set(current + 1);
                                         }
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

            // Online Payment Modal Dialog
            if *show_payment_modal.read() {
                Dialog {
                    open: *show_payment_modal.read(),
                    title: t("school-parent-billing-secure-payment", &locale),
                    onclose: move |_| show_payment_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        if *payment_success.read() {
                            div { class: "text-center py-6 space-y-3",
                                LucideIcon { name: "check-circle", class: "h-12 w-12 text-green-500 mx-auto" }
                                h4 { class: "text-sm font-bold text-foreground", {t("school-parent-billing-payment-success", &locale)} }
                                p { class: "text-xs text-muted-foreground", {crate::locales::t_with_args("school-parent-billing-payment-success-desc", &locale, &[("amount", &format_amount(*selected_pay_invoice_amount.read(), &locale))])} }
                                Button {
                                    class: "mt-4 px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground border-0 cursor-pointer",
                                    onclick: move |_| show_payment_modal.set(false),
                                    "Close"
                                }
                            }
                        } else if *payment_processing.read() {
                            div { class: "text-center py-12 space-y-3",
                                LucideIcon { name: "loader-2", class: "h-10 w-10 text-primary animate-spin mx-auto" }
                                p { class: "text-xs text-muted-foreground", "Processing secure payment transaction with Stripe gateway..." }
                            }
                        } else {
                            div { class: "space-y-4",
                                div { class: "p-3 bg-muted/20 border border-border/40 rounded-xl",
                                    div { class: "text-xs font-semibold text-muted-foreground", "Invoice Title" }
                                    div { class: "text-sm font-bold text-foreground mt-0.5", "{selected_pay_invoice_title}" }
                                    div { class: "text-xs font-semibold text-muted-foreground mt-2", "Amount Due" }
                                    div { class: "text-base font-black text-primary mt-0.5", "{format_amount(*selected_pay_invoice_amount.read(), &locale)}" }
                                }
                                div { class: "grid gap-1.5",
                                    span { class: "font-bold text-foreground text-xs", {t("school-parent-billing-cardholder", &locale)} }
                                    Input {
                                        placeholder: "Jane Doe",
                                        value: "Jane Doe",
                                    }
                                }
                                div { class: "grid gap-1.5",
                                    span { class: "font-bold text-foreground text-xs", {t("school-parent-billing-card-number", &locale)} }
                                    Input {
                                        placeholder: "4242 •••• •••• 4242",
                                        value: "{card_number}",
                                        oninput: move |evt: FormEvent| card_number.set(evt.value()),
                                    }
                                }
                                div { class: "grid grid-cols-2 gap-4",
                                    div { class: "grid gap-1.5",
                                        span { class: "font-bold text-foreground text-xs", {t("school-parent-billing-expiry", &locale)} }
                                        Input {
                                            placeholder: "MM/YY",
                                            value: "{card_expiry}",
                                            oninput: move |evt: FormEvent| card_expiry.set(evt.value()),
                                        }
                                    }
                                    div { class: "grid gap-1.5",
                                        span { class: "font-bold text-foreground text-xs", {t("school-parent-billing-cvc", &locale)} }
                                        Input {
                                            placeholder: "123",
                                            value: "{card_cvc}",
                                            oninput: move |evt: FormEvent| card_cvc.set(evt.value()),
                                        }
                                    }
                                }
                                div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                                    Button {
                                        class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground border-0 cursor-pointer",
                                        onclick: move |_| show_payment_modal.set(false),
                                        "Cancel"
                                    }
                                    Button {
                                        class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground border-0 cursor-pointer font-bold",
                                        onclick: {
                                            let uid_c = user_id.clone();
                                            let ws_c = ws_id.clone();
                                            let role_c = current_role.clone();
                                            let mut payment_processing_c = payment_processing;
                                            let mut payment_success_c = payment_success;
                                            let db_trigger_c = db_trigger;
                                            move |_| {
                                                if let Some(inv_id_val) = selected_pay_invoice_id.read().clone() {
                                                    payment_processing_c.set(true);
                                                    let u = uid_c.clone();
                                                    let w = ws_c.clone();
                                                    let r = role_c.clone();
                                                    let mut db_t = db_trigger_c.clone();
                                                    spawn(async move {
                                                        // Wait 1.5s to simulate Stripe delay
                                                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                                                         let proof = yntra_core::ZkCryptoTrust::new()
                                                             .generate_role_proof(state.get_passkey_seed(), u.clone(), r.clone())
                                                             .ok();
                                                        let _ = record_school_payment(u, w, inv_id_val, "Stripe Credit Card".to_string(), proof).await;
                                                        payment_processing_c.set(false);
                                                        payment_success_c.set(true);
                                                        let current = *db_t.read();
                                                        db_t.set(current + 1);
                                                    });
                                                }
                                            }
                                        },
                                        {t("school-parent-billing-confirm-pay", &locale)}
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

