use crate::components;
use dioxus::prelude::*;
use yntra_core::StudentProfile;

#[derive(Props, Clone)]
pub struct LibraryDashboardProps {
    pub active_user_id: String,
    pub students: Vec<StudentProfile>,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for LibraryDashboardProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn LibraryDashboard(props: LibraryDashboardProps) -> Element {
    let active_user_id = props.active_user_id.clone();
    let students = props.students.clone();
    let workspace_id = props.workspace_id.clone();
    let mut db_trigger = props.db_trigger;
    let db_trig = *db_trigger.read();

    // Query books
    let uid_books = active_user_id.clone();
    let books_res = use_resource(move || {
        let _ = db_trig;
        let u = uid_books.clone();
        async move {
            yntra_core::get_library_books(u).await.unwrap_or_default()
        }
    });
    let books = books_res.read().clone().unwrap_or_default();

    // Query all lending logs
    let uid = active_user_id.clone();
    let lending_logs_res = use_resource(move || {
        let _ = db_trig;
        let u = uid.clone();
        async move {
            yntra_core::get_library_lending_logs(u, None).await.unwrap_or_default()
        }
    });
    let lending_logs = lending_logs_res.read().clone().unwrap_or_default();

    // Selected book state
    let mut selected_book_id = use_signal(|| books.first().map(|b| b.id.clone()));
    let selected_book = books.iter().find(|b| Some(b.id.clone()) == *selected_book_id.read()).cloned();

    // Form states: New Book
    let mut book_title = use_signal(String::new);
    let mut book_author = use_signal(String::new);
    let mut book_isbn = use_signal(String::new);
    let mut book_copies = use_signal(|| 1);

    // Form states: Checkout
    let mut checkout_student_id = use_signal(|| students.first().map(|s| s.id.clone()).unwrap_or_default());
    let mut checkout_due_date = use_signal(String::new);

    // Filter books by search query
    let mut search_query = use_signal(String::new);
    let query = search_query.read().to_lowercase();
    let filtered_books: Vec<_> = books.iter()
        .filter(|b| b.title.to_lowercase().contains(&query) || b.author.to_lowercase().contains(&query) || b.isbn.contains(&query))
        .cloned()
        .collect();

    rsx! {
        div { class: "flex flex-col gap-6",


            // Main Grid
            div { class: "grid gap-6 lg:grid-cols-3 items-start",
                // Left Panel: Catalog List & Search
                div { class: "lg:col-span-1 flex flex-col gap-5",
                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                        h3 { class: "text-xs font-black uppercase text-muted-foreground tracking-wider m-0 pb-2 border-b border-border/30",
                            "Search Catalog"
                        }
                        
                        input {
                            r#type: "text",
                            placeholder: "Search by title, author, isbn...",
                            class: "yntra-input text-xs w-full",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                        }
                        
                        div { class: "flex flex-col gap-2 max-h-[300px] overflow-y-auto",
                            if filtered_books.is_empty() {
                                p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", "No books match search." }
                            } else {
                                for b in filtered_books.iter() {
                                    {
                                        let b_id = b.id.clone();
                                        let is_selected = Some(b_id.clone()) == *selected_book_id.read();
                                        let bg_cls = if is_selected { "bg-primary/10 border-primary text-primary" } else { "bg-sidebar/40 border-border/20 text-muted-foreground hover:bg-muted" };
                                        
                                        let first_char = b.title.chars().next().unwrap_or('B').to_ascii_uppercase();
                                        let gradient_cls = match first_char {
                                            'A'..='G' => "from-rose-500 to-orange-500",
                                            'H'..='N' => "from-emerald-500 to-teal-500",
                                            'O'..='U' => "from-sky-500 to-indigo-500",
                                            _ => "from-purple-500 to-pink-500",
                                        };

                                        rsx! {
                                            button {
                                                key: "{b_id}",
                                                class: "w-full text-left p-2.5 rounded-xl border text-xs font-semibold transition-all flex justify-between items-center {bg_cls}",
                                                onclick: move |_| selected_book_id.set(Some(b_id.clone())),
                                                div { class: "flex items-center gap-2.5",
                                                    div { class: "w-7 h-9 rounded bg-gradient-to-br {gradient_cls} shadow flex items-center justify-center border border-white/10 shrink-0 font-black text-white text-[10px]",
                                                        "{first_char}"
                                                    }
                                                    div { class: "flex flex-col gap-0.5",
                                                        span { class: "text-foreground font-black leading-tight", "{b.title}" }
                                                        span { class: "opacity-80 text-[10px]", "By: {b.author}" }
                                                    }
                                                }
                                                span { class: "text-[10px] font-bold px-1.5 py-0.5 bg-sidebar/80 rounded border border-border/30",
                                                    "{b.copies_available} / {b.total_copies}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Register Book Form Card
                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                        h3 { class: "text-xs font-black uppercase text-muted-foreground tracking-wider m-0 flex items-center gap-2",
                            components::LucideIcon { name: "plus", size: "14", class: "text-primary" }
                            "Register New Book"
                        }
                        
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Book Title" }
                            input {
                                r#type: "text",
                                placeholder: "e.g. Clean Code",
                                class: "yntra-input text-xs w-full",
                                value: "{book_title}",
                                oninput: move |e| book_title.set(e.value()),
                            }
                        }
                        
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Author" }
                            input {
                                r#type: "text",
                                placeholder: "e.g. Robert C. Martin",
                                class: "yntra-input text-xs w-full",
                                value: "{book_author}",
                                oninput: move |e| book_author.set(e.value()),
                            }
                        }

                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "ISBN" }
                            input {
                                r#type: "text",
                                placeholder: "978-...",
                                class: "yntra-input text-xs w-full",
                                value: "{book_isbn}",
                                oninput: move |e| book_isbn.set(e.value()),
                            }
                        }

                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Total Copies" }
                            input {
                                r#type: "number",
                                class: "yntra-input text-xs w-full",
                                value: "{book_copies}",
                                oninput: move |e| book_copies.set(e.value().parse::<i32>().unwrap_or(1)),
                            }
                        }

                        button {
                            class: "yntra-btn text-xs font-bold py-2 w-full mt-2 flex items-center justify-center gap-1.5 shadow-sm",
                            onclick: {
                                let ws = workspace_id.clone();
                                let uid = active_user_id.clone();
                                move |_| {
                                    let ws_c = ws.clone();
                                    let title = book_title.read().clone();
                                    let author = book_author.read().clone();
                                    let isbn = book_isbn.read().clone();
                                    let copies = *book_copies.read();
                                    let uid_c = uid.clone();
                                    
                                    spawn(async move {
                                        if yntra_core::save_library_book(uid_c, None, ws_c, title, author, isbn, copies, copies).await.is_ok() {
                                            book_title.set(String::new());
                                            book_author.set(String::new());
                                            book_isbn.set(String::new());
                                            book_copies.set(1);
                                            let current = *db_trigger.read();
                                            db_trigger.set(current + 1);
                                        }
                                    });
                                }
                            },
                            components::LucideIcon { name: "plus", size: "14" }
                            "Register Book"
                        }
                    }
                }

                // Right Panel: Selected Book and Checkout Log Console
                div { class: "lg:col-span-2 flex flex-col gap-6",
                    if let Some(ref book) = selected_book {
                        div { class: "flex flex-col gap-6",
                            // Selected Book header
                            div { class: "flex justify-between items-start p-5 border border-border/40 bg-sidebar/10 rounded-xl flex-wrap gap-4",
                                div { class: "flex items-start gap-3",
                                    div { class: "p-2.5 rounded-lg bg-primary/10 text-primary mt-0.5",
                                        components::LucideIcon { name: "book-open", size: "20" }
                                    }
                                    div { class: "flex flex-col gap-0.5",
                                        h3 { class: "text-base font-black text-foreground m-0", "{book.title}" }
                                        span { class: "text-xs text-muted-foreground font-semibold", "By {book.author} | ISBN: {book.isbn}" }
                                    }
                                }
                                
                                div { class: "flex flex-col items-end gap-0.5",
                                    span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Available Inventory" }
                                    span { class: "text-xl font-black text-foreground", "{book.copies_available} / {book.total_copies} available" }
                                }
                            }

                            // Sub-grids: Checkout Form & Current Loans
                            div { class: "grid gap-6 md:grid-cols-3 items-start",
                                // Checkout Form
                                div { class: "md:col-span-1 flex flex-col gap-4",
                                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-3.5 shadow-sm",
                                        h3 { class: "text-xs font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                            components::LucideIcon { name: "plus", size: "14", class: "text-primary" }
                                            "Checkout Copy"
                                        }

                                        if book.copies_available <= 0 {
                                            p { class: "text-xs text-rose-400 italic text-center p-4 border border-rose-500/20 bg-rose-500/5 rounded-xl m-0",
                                                "All copies checked out."
                                            }
                                        } else {
                                            div { class: "flex flex-col gap-3",
                                                // Roster selector
                                                div { class: "flex flex-col gap-1",
                                                    label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Borrowing Student" }
                                                    select {
                                                        class: "yntra-input py-1.5 text-xs bg-sidebar w-full",
                                                        value: "{checkout_student_id}",
                                                        onchange: move |e| checkout_student_id.set(e.value()),
                                                        for s in students.iter() {
                                                            option { value: "{s.id}", "{s.first_name} {s.last_name}" }
                                                        }
                                                    }
                                                }

                                                // Return due date
                                                div { class: "flex flex-col gap-1",
                                                    label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Due Return Date" }
                                                    input {
                                                        r#type: "date",
                                                        class: "yntra-input text-xs w-full",
                                                        value: "{checkout_due_date}",
                                                        oninput: move |e| checkout_due_date.set(e.value()),
                                                    }
                                                }

                                                button {
                                                    class: "yntra-btn text-xs font-bold py-2 w-full mt-1.5 flex items-center justify-center gap-1.5 shadow-sm",
                                                    onclick: {
                                                        let ws = workspace_id.clone();
                                                        let b_id = book.id.clone();
                                                        let uid = active_user_id.clone();
                                                        move |_| {
                                                            let ws_c = ws.clone();
                                                            let b_id_c = b_id.clone();
                                                            let s_id = checkout_student_id.read().clone();
                                                            let due = checkout_due_date.read().clone();
                                                            let uid_c = uid.clone();
                                                            let now_str = std::time::SystemTime::now()
                                                                .duration_since(std::time::UNIX_EPOCH)
                                                                .map(|d| {
                                                                    let secs = d.as_secs() as i64;
                                                                    let mut days = secs / 86400;
                                                                    let mut year = 1970;
                                                                    loop {
                                                                        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                                        let days_in_year = if leap { 366 } else { 365 };
                                                                        if days >= days_in_year {
                                                                            days -= days_in_year;
                                                                            year += 1;
                                                                        } else {
                                                                            break;
                                                                        }
                                                                    }
                                                                    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                                    let month_lengths = if leap {
                                                                        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                                    } else {
                                                                        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                                    };
                                                                    let mut month = 1;
                                                                    for &length in month_lengths.iter() {
                                                                        if days >= length {
                                                                            days -= length;
                                                                            month += 1;
                                                                        } else {
                                                                            break;
                                                                        }
                                                                    }
                                                                    let day = days + 1;
                                                                    format!("{:04}-{:02}-{:02}", year, month, day)
                                                                })
                                                                .unwrap_or_else(|_| "2026-07-04".to_string());
                                                                
                                                            spawn(async move {
                                                                if yntra_core::checkout_library_book(uid_c, ws_c, b_id_c, s_id, now_str, due).await.is_ok() {
                                                                    checkout_due_date.set(String::new());
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                }
                                                            });
                                                        }
                                                    },
                                                    components::LucideIcon { name: "check", size: "14" }
                                                    "Confirm Checkout"
                                                }
                                            }
                                        }
                                    }
                                }

                                // Current active loans ledger for this book
                                div { class: "md:col-span-2 flex flex-col gap-4",
                                    components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                                        h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                            components::LucideIcon { name: "time", size: "16", class: "text-primary" }
                                            "Active Loans for this Asset"
                                        }

                                        {
                                            let book_loans: Vec<_> = lending_logs.iter()
                                                .filter(|l| l.book_id == book.id && l.status != "returned")
                                                .collect();
                                            
                                            rsx! {
                                                if book_loans.is_empty() {
                                                    p { class: "text-xs text-muted-foreground italic text-center p-8 m-0", "No active loans registered for this book." }
                                                } else {
                                                    div { class: "flex flex-col gap-3.5",
                                                        for loan in book_loans.iter() {
                                                            {
                                                                let student_name = students.iter()
                                                                    .find(|s| s.id == loan.student_id)
                                                                    .map(|s| format!("{} {}", s.first_name, s.last_name))
                                                                    .unwrap_or_else(|| "Unknown Student".to_string());
                                                                let l_id = loan.id.clone();
                                                                let is_overdue = loan.status == "overdue";
                                                                
                                                                rsx! {
                                                                    div { key: "{l_id}", class: "border border-border/30 p-4 rounded-xl bg-sidebar/40 flex flex-col gap-3",
                                                                        div { class: "flex justify-between items-start flex-wrap gap-2",
                                                                            div { class: "flex flex-col gap-0.5",
                                                                                span { class: "text-xs font-black text-foreground", "{student_name}" }
                                                                                span { class: "text-[10px] text-muted-foreground font-semibold", "Borrowed: {loan.checked_out_at} | Due: {loan.due_date}" }
                                                                            }
                                                                            
                                                                            if is_overdue {
                                                                                span { class: "px-2 py-0.5 rounded text-[9px] font-black bg-rose-500/10 text-rose-400 border border-rose-500/15 shadow-sm animate-pulse", "OVERDUE" }
                                                                            } else {
                                                                                span { class: "px-2 py-0.5 rounded text-[9px] font-black bg-primary/10 text-primary border border-primary/15 shadow-sm", "Active Loan" }
                                                                            }
                                                                        }
                                                                        
                                                                        div { class: "flex justify-end border-t border-border/20 pt-2",
                                                                            button {
                                                                                class: "px-3 py-1 rounded text-[10px] font-bold bg-emerald-500 text-white hover:bg-emerald-600 transition-all flex items-center gap-1 shadow-sm",
                                                                                onclick: {
                                                                                    let log_id_c = l_id.clone();
                                                                                    let uid = active_user_id.clone();
                                                                                    move |_| {
                                                                                        let uid_c = uid.clone();
                                                                                        let l_id_clone = log_id_c.clone();
                                                                                        let now_str = std::time::SystemTime::now()
                                                                                            .duration_since(std::time::UNIX_EPOCH)
                                                                                            .map(|d| {
                                                                                                let secs = d.as_secs() as i64;
                                                                                                let mut days = secs / 86400;
                                                                                                let mut year = 1970;
                                                                                                loop {
                                                                                                    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                                                                    let days_in_year = if leap { 366 } else { 365 };
                                                                                                    if days >= days_in_year {
                                                                                                        days -= days_in_year;
                                                                                                        year += 1;
                                                                                                    } else {
                                                                                                        break;
                                                                                                    }
                                                                                                }
                                                                                                let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                                                                let month_lengths = if leap {
                                                                                                    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                                                                } else {
                                                                                                    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                                                                };
                                                                                                let mut month = 1;
                                                                                                for &length in month_lengths.iter() {
                                                                                                    if days >= length {
                                                                                                        days -= length;
                                                                                                        month += 1;
                                                                                                    } else {
                                                                                                        break;
                                                                                                    }
                                                                                                }
                                                                                                let day = days + 1;
                                                                                                format!("{:04}-{:02}-{:02}", year, month, day)
                                                                                            })
                                                                                            .unwrap_or_else(|_| "2026-07-04".to_string());
                                                                                        
                                                                                        spawn(async move {
                                                                                            if yntra_core::return_library_book(uid_c, l_id_clone, now_str).await.is_ok() {
                                                                                                let current = *db_trigger.read();
                                                                                                db_trigger.set(current + 1);
                                                                                            }
                                                                                        });
                                                                                    }
                                                                                },
                                                                                components::LucideIcon { name: "check", size: "12" }
                                                                                "Return Asset"
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
                        p { class: "text-xs text-muted-foreground italic text-center p-8 bg-sidebar/10 rounded-xl", "Please select a book in the catalog to view lending actions." }
                    }
                }
            }
        }
    }
}
