use dioxus::prelude::*;
use crate::components;
use yntra_core::{LibraryLendingLog, LibraryBook};

#[derive(Props, Clone)]
pub struct LibraryTabProps {
    pub library_logs: Vec<LibraryLendingLog>,
    pub library_books: Vec<LibraryBook>,
}

impl PartialEq for LibraryTabProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn LibraryTab(props: LibraryTabProps) -> Element {
    let library_logs = props.library_logs;
    let library_books = props.library_books;

    let active_count = library_logs.iter().filter(|l| l.status != "returned").count();
    let overdue_count = library_logs.iter().filter(|l| l.status == "overdue").count();

    rsx! {
        div { class: "grid gap-6 md:grid-cols-3 items-start",
            // Left: Active loans count summary
            div { class: "md:col-span-1 flex flex-col gap-4",
                components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                    h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "book-open", size: "16", class: "text-primary" }
                        "Library Status"
                    }
                    
                    div { class: "flex flex-col gap-3",
                        div { class: "p-4 rounded-xl bg-sidebar/40 border border-border/20 flex flex-col gap-1 text-center",
                            span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Active Loans" }
                            span { class: "text-2xl font-black text-foreground", "{active_count}" }
                        }
                        
                        if overdue_count > 0 {
                            div { class: "p-3 rounded-xl border border-rose-500/20 bg-rose-500/5 flex flex-col items-center text-center gap-1 text-rose-400 font-bold",
                                components::LucideIcon { name: "time", size: "18", class: "animate-pulse" }
                                span { class: "text-[10px] uppercase tracking-wider", "Overdue Books" }
                                span { class: "text-base font-black", "{overdue_count} OVERDUE" }
                            }
                        }
                    }
                }
            }

            // Right: Book loans registry
            div { class: "md:col-span-2 flex flex-col gap-4",
                components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                    h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "book-open", size: "16", class: "text-primary" }
                        "Asset Loan Ledger"
                    }

                    if library_logs.is_empty() {
                        p { class: "text-xs text-muted-foreground italic text-center p-8 m-0", "No books borrowed by this student." }
                    } else {
                        div { class: "flex flex-col gap-4",
                            for loan in library_logs.iter() {
                                {
                                    let book_title = library_books.iter()
                                        .find(|b| b.id == loan.book_id)
                                        .map(|b| b.title.clone())
                                        .unwrap_or_else(|| "Unknown Book".to_string());
                                    let is_returned = loan.status == "returned";
                                    
                                    rsx! {
                                        div { key: "{loan.id}", class: "border border-border/30 p-4 rounded-xl bg-white/[0.01] flex flex-col gap-2.5",
                                            div { class: "flex justify-between items-center",
                                                div { class: "flex flex-col gap-0.5",
                                                    span { class: "text-sm font-black text-foreground", "{book_title}" }
                                                    span { class: "text-[10px] text-muted-foreground font-semibold", "Borrowed: {loan.checked_out_at} | Due: {loan.due_date}" }
                                                }
                                                div { class: "flex items-center gap-2.5",
                                                    match loan.status.as_str() {
                                                        "returned" => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Returned" } },
                                                        "overdue" => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-rose-500/10 text-rose-400 border border-rose-500/15 shadow-sm animate-pulse", "Overdue" } },
                                                        _ => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-primary/10 text-primary border border-primary/15 shadow-sm", "Active" } }
                                                    }
                                                }
                                            }
                                            
                                            if is_returned {
                                                if let Some(ref ret_at) = loan.returned_at {
                                                    div { class: "text-[10px] text-muted-foreground bg-sidebar/30 p-2 rounded border border-border/20 font-medium",
                                                        "Returned and checked in on {ret_at}"
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
