use dioxus::prelude::*;
use yntra_core::LibraryBook;
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct BookRegistrationCardProps {
    pub workspace_id: String,
    pub active_user_id: String,
    pub db_trigger: Signal<u32>,
}

#[component]
pub fn BookRegistrationCard(props: BookRegistrationCardProps) -> Element {
    let mut db_trigger = props.db_trigger;
    let mut book_title = use_signal(String::new);
    let mut book_author = use_signal(String::new);
    let mut book_isbn = use_signal(String::new);
    let mut book_copies = use_signal(|| 1);

    rsx! {
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
                    let ws = props.workspace_id.clone();
                    let uid = props.active_user_id.clone();
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
}

#[derive(Props, Clone, PartialEq)]
pub struct BookCatalogListProps {
    pub books: Vec<LibraryBook>,
    pub selected_book_id: Signal<Option<String>>,
}

#[component]
pub fn BookCatalogList(props: BookCatalogListProps) -> Element {
    let mut selected_book_id = props.selected_book_id;
    let mut search_query = use_signal(String::new);
    let query = search_query.read().to_lowercase();
    let filtered_books: Vec<_> = props.books.iter()
        .filter(|b| b.title.to_lowercase().contains(&query) || b.author.to_lowercase().contains(&query) || b.isbn.contains(&query))
        .cloned()
        .collect();

    rsx! {
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
    }
}
