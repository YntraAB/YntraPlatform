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
    get_student_submissions, save_submission, get_timetable_slots, save_timetable_slot, save_library_book, LibraryBook,
    get_parent_students, SchoolInvoice, HealthRecord, HealthIncident, StudentProfile, TermGrade, ReportCard,
    Submission, TimetableSlot, reserve_book, renew_book,
};

use super::SchoolViewProps;

#[component]
pub fn LibraryView(props: SchoolViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut db_trigger = state.trigger_school;
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();
    let locale = props.locale.clone();

    let mut search_query = use_signal(String::new);

    // Local checkout states
    let mut show_checkout_modal = use_signal(|| false);
    let mut checkout_book_id = use_signal(String::new);
    let mut checkout_student_id = use_signal(String::new);
    let mut checkout_due = use_signal(|| "2026-08-01".to_string());

    let mut show_reserve_modal = use_signal(|| false);
    let mut reserve_book_id = use_signal(String::new);
    let mut reserve_student_id = use_signal(String::new);

    let mut context_menu_open = use_signal(|| false);
    let mut context_menu_pos = use_signal(|| (0, 0));
    let mut context_menu_book = use_signal(|| Option::<LibraryBook>::None);

    let mut show_edit_book_modal = use_signal(|| false);
    let mut edit_book_id = use_signal(String::new);
    let mut edit_book_title = use_signal(String::new);
    let mut edit_book_author = use_signal(String::new);
    let mut edit_book_isbn = use_signal(String::new);
    let mut edit_book_total_copies = use_signal(String::new);
    let mut edit_book_copies_available = use_signal(|| 0);

    let db_trig_val = *db_trigger.read();
    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let books_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { get_library_books(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let logs_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move { get_library_lending_logs(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone3 = user_id.clone();
    let ws_id_clone3 = ws_id.clone();
    let students_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone3.clone();
        let ws = ws_id_clone3.clone();
        async move { get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone_p = user_id.clone();
    let ws_id_clone_p = ws_id.clone();
    let parent_students_res = use_resource(move || {
        let _ = db_trig_val;
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

    let current_role = state.active_user_role.read().clone();
    let books_raw = books_res.read().clone().unwrap_or_default();
    let books: Vec<yntra_core::LibraryBook> = {
        let q = search_query.read().to_lowercase();
        if q.is_empty() {
            books_raw
        } else {
            books_raw.into_iter()
                .filter(|b| b.title.to_lowercase().contains(&q) || b.author.to_lowercase().contains(&q) || b.isbn.contains(&q))
                .collect()
        }
    };
    let logs_raw = logs_res.read().clone().unwrap_or_default();
    
    let students = if current_role == "parent" || current_role == "role-school-parent" {
        parent_students_res.read().clone().unwrap_or_default()
    } else {
        students_res.read().clone().unwrap_or_default()
    };

    let logs = if current_role == "parent" || current_role == "role-school-parent" {
        let s_id = selected_student_id.read().clone();
        logs_raw.into_iter().filter(|lg| lg.student_id == s_id).collect::<Vec<_>>()
    } else if current_role == "student" || current_role == "role-school-student" {
        let student_names: std::collections::HashSet<String> = students.iter()
            .filter(|s| s.user_id.as_ref() == Some(&user_id))
            .map(|s| format!("{} {}", s.first_name, s.last_name))
            .collect();
        logs_raw.into_iter().filter(|lg| student_names.contains(&lg.student_name)).collect::<Vec<_>>()
    } else {
        logs_raw
    };

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "border-b border-border pb-4",
                h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                    LucideIcon { name: "library", class: "h-6 w-6 text-primary" }
                    "Library Lending Catalog"
                }
                p { class: "text-xs text-muted-foreground m-0 mt-1", "Manage catalog records, checkout books to students, and track return lending logs." }
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

            // Grid for book catalog vs checkout logs
            div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
                // Library Catalog Table Card
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { "{t(\"school-library-catalog\", &locale)}" }
                        CardDescription { "{t(\"school-library-catalog-desc\", &locale)}" }
                        div { class: "mt-2",
                            Input {
                                placeholder: format!("{}...", t("school-library-search-placeholder", &locale)),
                                value: "{search_query}",
                                oninput: move |e: FormEvent| search_query.set(e.value()),
                            }
                        }
                    }
                    CardContent {
                        if books.is_empty() {
                            div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "{t(\"school-library-no-books\", &locale)}" }
                        } else {
                            div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                for b in books.iter() {
                                    {
                                        let current_role = current_role.clone();
                                        let b_c = b.clone();
                                        let b_context = b.clone();
                                        let book_id = b.id.clone();
                                        let is_available = b.copies_available > 0;
                                        rsx! {
                                            div {
                                                key: "{book_id}",
                                                class: "p-4 flex justify-between items-center",
                                                oncontextmenu: move |evt| {
                                                    if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                                                        evt.prevent_default();
                                                        let coords = evt.client_coordinates();
                                                        context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                        context_menu_book.set(Some(b_context.clone()));
                                                        context_menu_open.set(true);
                                                    }
                                                },
                                                div {
                                                    div { class: "font-semibold text-sm text-foreground", "{b_c.title}" }
                                                    div { class: "text-xs text-muted-foreground mt-0.5", "{t(\"school-library-author\", &locale)}: {b_c.author} | ISBN: {b_c.isbn}" }
                                                    div { class: "text-[10px] text-muted-foreground mt-2", "{t(\"school-library-copies\", &locale)}: {b_c.copies_available} {t(\"school-library-available-of\", &locale)} {b_c.total_copies}" }
                                                }
                                                if is_available {
                                                    if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                                                        Button {
                                                            class: "text-xs h-8 px-3 rounded-lg border border-primary text-primary hover:bg-primary/5 font-semibold",
                                                            onclick: move |_| {
                                                                checkout_book_id.set(book_id.clone());
                                                                show_checkout_modal.set(true);
                                                            },
                                                            "{t(\"school-library-checkout-btn\", &locale)}"
                                                        }
                                                    } else {
                                                        Button {
                                                            class: "text-xs h-8 px-3 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 font-semibold border-0 cursor-pointer",
                                                            onclick: {
                                                                let book_id_c = book_id.clone();
                                                                let uid_c = user_id.clone();
                                                                let ws_c = ws_id.clone();
                                                                let role_c = current_role.clone();
                                                                let students_c = students.clone();
                                                                let mut db_trigger = db_trigger.clone();
                                                                let mut show_reserve_modal = show_reserve_modal;
                                                                let mut reserve_book_id = reserve_book_id;
                                                                let mut reserve_student_id = reserve_student_id;
                                                                move |_| {
                                                                    if (role_c == "parent" || role_c == "role-school-parent") && students_c.len() > 1 {
                                                                        reserve_book_id.set(book_id_c.clone());
                                                                        reserve_student_id.set(students_c[0].id.clone());
                                                                        show_reserve_modal.set(true);
                                                                    } else {
                                                                        let target_student = if role_c == "student" || role_c == "role-school-student" {
                                                                            students_c.iter().find(|s| s.user_id.as_ref() == Some(&uid_c)).map(|s| s.id.clone())
                                                                        } else {
                                                                            students_c.first().map(|s| s.id.clone())
                                                                        };
                                                                        if let Some(s_id) = target_student {
                                                                             let proof = yntra_core::ZkCryptoTrust::new()
                                                                                 .generate_role_proof(state.get_passkey_seed(), uid_c.clone(), role_c.clone())
                                                                                 .ok();
                                                                            let u = uid_c.clone();
                                                                            let w = ws_c.clone();
                                                                            let b = book_id_c.clone();
                                                                            let mut db_t = db_trigger.clone();
                                                                            spawn(async move {
                                                                                let _ = reserve_book(u, w, b, s_id, proof).await;
                                                                                let cur = *db_t.read();
                                                                                db_t.set(cur + 1);
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                            },
                                                            {t("school-library-reserve-btn", &locale)}
                                                        }
                                                    }
                                                } else {
                                                    span { class: "text-xs text-red-500 font-medium", "{t(\"school-library-unavailable\", &locale)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Lending Logs Card
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { "Lending Logs" }
                        CardDescription { "Active book checkouts and status logs" }
                    }
                    CardContent {
                        if logs.is_empty() {
                            div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No lending logs registered." }
                        } else {
                            div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                for lg in logs.iter() {
                                    {
                                        let current_role = current_role.clone();
                                        let log_id = lg.id.clone();
                                        let is_borrowed = lg.status == "borrowed";
                                        let uid = user_id.clone();
                                        let ws = ws_id.clone();
                                        rsx! {
                                            div { key: "{log_id}", class: "p-4 flex justify-between items-center",
                                                div {
                                                    div { class: "font-semibold text-sm text-foreground", "{lg.book_title}" }
                                                    div { class: "text-xs text-muted-foreground mt-0.5", "Borrower: {lg.student_name} | Due: {lg.due_date}" }
                                                    div { class: "text-[10px] mt-1.5",
                                                        span { class: format!(
                                                            "text-[9px] font-bold uppercase rounded px-1.5 py-0.5 {}",
                                                            if is_borrowed { "bg-amber-500/10 text-amber-600" } else { "bg-green-500/10 text-green-600" }
                                                        ),
                                                            "{lg.status}"
                                                        }
                                                    }
                                                }
                                                if is_borrowed {
                                                    if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                                                        Button {
                                                            class: "text-xs h-8 px-3 rounded-lg border border-border text-foreground hover:bg-muted font-medium",
                                                             onclick: {
                                                                 let uid_c = uid.clone();
                                                                 let ws_c = ws.clone();
                                                                 let log_id_c = log_id.clone();
                                                                 let state = state;
                                                                 move |_| {
                                                                     let role = state.active_user_role.read().clone();
                                                                     let u_id = state.active_user_id.read().clone();
                                                                      let proof = yntra_core::ZkCryptoTrust::new()
                                                                          .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                                                          .ok();
                                                                     let u = uid_c.clone();
                                                                     let w = ws_c.clone();
                                                                     let l = log_id_c.clone();
                                                                     spawn(async move {
                                                                         let _ = return_book(u, w, l, proof).await;
                                                                     });
                                                                     let current = *db_trigger.read();
                                                                     db_trigger.set(current + 1);
                                                                 }
                                                             },
                                                            "Return Book"
                                                        }
                                                    } else {
                                                        Button {
                                                            class: "text-xs h-8 px-3 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 font-semibold border-0 cursor-pointer",
                                                            onclick: {
                                                                let uid_c = uid.clone();
                                                                let ws_c = ws.clone();
                                                                let log_id_c = log_id.clone();
                                                                let state = state;
                                                                move |_| {
                                                                    let role = state.active_user_role.read().clone();
                                                                    let u_id = state.active_user_id.read().clone();
                                                                    let proof = yntra_core::ZkCryptoTrust::new()
                                                                        .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                                                        .ok();
                                                                    let u = uid_c.clone();
                                                                    let w = ws_c.clone();
                                                                    let l = log_id_c.clone();
                                                                    spawn(async move {
                                                                        let _ = renew_book(u, w, l, proof).await;
                                                                    });
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                }
                                                            },
                                                            "{t(\"school-library-renew-btn\", &locale)}"
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

            // Reserve Dialog Modal for parents with multiple children
            if *show_reserve_modal.read() {
                Dialog {
                    open: *show_reserve_modal.read(),
                    title: t("school-parent-select-child", &locale),
                    onclose: move |_| show_reserve_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", {t("school-parent-select-child", &locale)} }
                            select {
                                class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50",
                                value: reserve_student_id.read().clone(),
                                onchange: move |evt: FormEvent| reserve_student_id.set(evt.value()),
                                for s in students.iter() {
                                    option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                                }
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground border border-border/40 cursor-pointer",
                                onclick: move |_| show_reserve_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground font-bold border-0 cursor-pointer",
                                onclick: {
                                     let uid = user_id.clone();
                                     let ws = ws_id.clone();
                                     let role_c = current_role.clone();
                                     let state = state.clone();
                                     let mut db_trigger = db_trigger.clone();
                                     move |_| {
                                          let proof = yntra_core::ZkCryptoTrust::new()
                                              .generate_role_proof(state.get_passkey_seed(), uid.clone(), role_c.clone())
                                              .ok();
                                          let bk_id = reserve_book_id.read().clone();
                                          let std_id = reserve_student_id.read().clone();
                                          let uid_c = uid.clone();
                                          let ws_c = ws.clone();
                                          spawn(async move {
                                              let _ = reserve_book(uid_c, ws_c, bk_id, std_id, proof).await;
                                          });
                                          show_reserve_modal.set(false);
                                          let current = *db_trigger.read();
                                          db_trigger.set(current + 1);
                                     }
                                },
                                {t("school-library-reserve-btn", &locale)}
                            }
                        }
                    }
                }
            }

            // Checkout Dialog Modal
            if *show_checkout_modal.read() {
                Dialog {
                    open: *show_checkout_modal.read(),
                    title: "Checkout Book copy",
                    onclose: move |_| show_checkout_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Select Borrower Student" }
                            select {
                                class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none",
                                value: checkout_student_id.read().clone(),
                                onchange: move |evt: FormEvent| checkout_student_id.set(evt.value()),
                                option { value: "", "Choose student..." }
                                for s in students.iter() {
                                    option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                                }
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Due Date" }
                            Input {
                                value: checkout_due.read().clone(),
                                oninput: move |evt: FormEvent| checkout_due.set(evt.value()),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_checkout_modal.set(false),
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
                                              .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                              .ok();
                                          let bk_id = checkout_book_id.read().clone();
                                         let std_id = checkout_student_id.read().clone();
                                         let due_val = checkout_due.read().clone();
                                         let uid_c = uid.clone();
                                         let ws_c = ws.clone();
                                         spawn(async move {
                                             let _ = checkout_book(uid_c, ws_c, bk_id, std_id, due_val, proof).await;
                                         });
                                        show_checkout_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                },
                                "Log Checkout"
                            }
                        }
                    }
                }
            }

            // Edit Book Dialog Modal
            if *show_edit_book_modal.read() {
                Dialog {
                    open: *show_edit_book_modal.read(),
                    title: "Edit Book Details",
                    onclose: move |_| show_edit_book_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Title" }
                            Input {
                                value: edit_book_title.read().clone(),
                                oninput: move |evt: FormEvent| edit_book_title.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Author" }
                            Input {
                                value: edit_book_author.read().clone(),
                                oninput: move |evt: FormEvent| edit_book_author.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "ISBN" }
                            Input {
                                value: edit_book_isbn.read().clone(),
                                oninput: move |evt: FormEvent| edit_book_isbn.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Total Copies" }
                            Input {
                                value: edit_book_total_copies.read().clone(),
                                oninput: move |evt: FormEvent| edit_book_total_copies.set(evt.value()),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_edit_book_modal.set(false),
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
                                              .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                              .ok();
                                          let tot_copies = edit_book_total_copies.read().parse::<i32>().unwrap_or(1);
                                         let av = *edit_book_copies_available.read();
                                         let b = LibraryBook {
                                             id: edit_book_id.read().clone(),
                                             workspace_id: ws.clone(),
                                             title: edit_book_title.read().clone(),
                                             author: edit_book_author.read().clone(),
                                             isbn: edit_book_isbn.read().clone(),
                                             copies_available: av,
                                             total_copies: tot_copies,
                                             updated_at: 0,
                                         };
                                         let uid_c = uid.clone();
                                         spawn(async move {
                                             let _ = save_library_book(uid_c, b, proof).await;
                                         });
                                        show_edit_book_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                },
                                "Save Book"
                            }
                        }
                    }
                }
            }

            // Context Menu Overlay
            if let Some(book) = context_menu_book.read().clone() {
                {
                    let bk = book.clone();
                    let bk_edit = book.clone();
                    let bk_checkout = book.clone();

                    let active_log = logs.iter().find(|lg| lg.book_title == bk.title && lg.status == "borrowed").cloned();
                    let is_available = bk.copies_available > 0;

                    rsx! {
                        crate::components::ContextMenu {
                            open: *context_menu_open.read(),
                            x: context_menu_pos.read().0,
                            y: context_menu_pos.read().1,
                            onclose: move |_| context_menu_open.set(false),

                            if is_available {
                                button {
                                    class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                    onclick: move |_| {
                                        checkout_book_id.set(bk_checkout.id.clone());
                                        show_checkout_modal.set(true);
                                        context_menu_open.set(false);
                                    },
                                    crate::components::LucideIcon { name: "book-open", size: "14" }
                                    "Checkout Book"
                                }
                            }
                            if let Some(log) = active_log {
                                {
                                    let log_id = log.id.clone();
                                    let uid_c = user_id.clone();
                                    let ws_c = ws_id.clone();
                                    rsx! {
                                        button {
                                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                             onclick: move |_| {
                                                 let role = state.active_user_role.read().clone();
                                                 let u_id = state.active_user_id.read().clone();
                                                 let proof = yntra_core::ZkCryptoTrust::new()
                                                     .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                                     .ok();
                                                 let u = uid_c.clone();
                                                 let w = ws_c.clone();
                                                 let l = log_id.clone();
                                                 spawn(async move {
                                                     let _ = return_book(u, w, l, proof).await;
                                                 });
                                                context_menu_open.set(false);
                                                let current = *db_trigger.read();
                                                db_trigger.set(current + 1);
                                            },
                                            crate::components::LucideIcon { name: "book-copy", size: "14" }
                                            "Return Book"
                                        }
                                    }
                                }
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    edit_book_id.set(bk_edit.id.clone());
                                    edit_book_title.set(bk_edit.title.clone());
                                    edit_book_author.set(bk_edit.author.clone());
                                    edit_book_isbn.set(bk_edit.isbn.clone());
                                    edit_book_total_copies.set(format!("{}", bk_edit.total_copies));
                                    edit_book_copies_available.set(bk_edit.copies_available);
                                    show_edit_book_modal.set(true);
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "edit", size: "14" }
                                "Edit Book"
                            }
                        }
                    }
                }
            }
        }
    }
}

