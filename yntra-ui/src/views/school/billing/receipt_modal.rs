use dioxus::prelude::*;
use yntra_core::SchoolInvoice;
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct ReceiptModalDialogProps {
    pub selected_invoice_for_receipt: Signal<Option<SchoolInvoice>>,
}

#[component]
pub fn ReceiptModalDialog(props: ReceiptModalDialogProps) -> Element {
    let mut selected_invoice_for_receipt = props.selected_invoice_for_receipt;
    
    rsx! {
        if let Some(ref receipt) = *selected_invoice_for_receipt.read() {
            div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm animate-in fade-in duration-200",
                div { class: "w-full max-w-md p-6 bg-sidebar border border-border/50 rounded-2xl shadow-2xl flex flex-col gap-4 relative overflow-hidden animate-in scale-in duration-200",
                    // Decorative watermarked logo background
                    div { class: "absolute -right-16 -bottom-16 opacity-[0.03] text-primary",
                        components::LucideIcon { name: "award", size: "180" }
                    }
                    
                    div { class: "flex justify-between items-center pb-2 border-b border-border/20",
                        div { class: "flex items-center gap-2",
                            components::LucideIcon { name: "check-circle", size: "18", class: "text-emerald-400" }
                            h3 { class: "text-sm font-black uppercase text-foreground m-0 tracking-wider", "Transaction Receipt" }
                        }
                        button {
                            class: "p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-all",
                            onclick: move |_| selected_invoice_for_receipt.set(None),
                            components::LucideIcon { name: "x", size: "16" }
                        }
                    }
                    
                    // Receipt Body
                    div { class: "flex flex-col gap-4 text-xs font-semibold text-muted-foreground pt-2",
                        div { class: "flex justify-between items-center",
                            span { "Receipt Number:" }
                            span { class: "text-foreground font-black font-mono text-[10px]", "REC-{receipt.id.get(0..8).unwrap_or(\"...\")}" }
                        }
                        div { class: "flex justify-between items-center",
                            span { "Payment Date:" }
                            span { class: "text-foreground font-bold", "{receipt.paid_at.clone().unwrap_or_else(|| \"N/A\".to_string())}" }
                        }
                        div { class: "flex justify-between items-center",
                            span { "Payment Method:" }
                            span { class: "text-foreground font-bold capitalize", "Bank Transfer / Manual Record" }
                        }
                        
                        // Line items box
                        div { class: "bg-sidebar/40 p-4 rounded-xl border border-border/30 flex flex-col gap-2 mt-2",
                            div { class: "flex justify-between items-center text-[10px] font-black uppercase tracking-wider text-muted-foreground pb-1.5 border-b border-border/20",
                                span { "Description" }
                                span { "Amount" }
                            }
                            div { class: "flex justify-between items-center pt-1 font-bold text-foreground text-xs",
                                span { "{receipt.title}" }
                                span { "${receipt.amount:.2}" }
                            }
                        }
                        
                        // Total Box
                        div { class: "flex justify-between items-center text-sm pt-2 border-t border-border/20 font-black",
                            span { class: "text-foreground", "Total Paid:" }
                            span { class: "text-emerald-400 text-lg", "${receipt.amount:.2}" }
                        }
                    }
                    
                    div { class: "flex justify-end gap-2 mt-4 pt-2 border-t border-border/20 z-10",
                        button {
                            class: "yntra-btn text-xs py-1.5 px-4 font-bold bg-primary text-primary-foreground hover:bg-primary/80 transition-all",
                            onclick: move |_| selected_invoice_for_receipt.set(None),
                            "Done"
                        }
                    }
                }
            }
        }
    }
}
