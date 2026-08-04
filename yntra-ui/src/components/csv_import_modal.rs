use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use std::collections::HashMap;
use yntra_core::{CsvImportPreview, execute_csv_import, parse_and_preview_csv};

#[derive(Props, Clone, PartialEq)]
pub struct CsvImportModalProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub target_block_id: String,
    pub block_name: String,
    pub onimportcomplete: EventHandler<u32>,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn CsvImportModal(props: CsvImportModalProps) -> Element {
    let mut raw_csv_text = use_signal(|| String::new());
    let mut preview_state = use_signal(|| Option::<CsvImportPreview>::None);
    let mut mappings_state = use_signal(|| HashMap::<String, String>::new());
    let mut is_parsing = use_signal(|| false);
    let mut is_executing = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut success_msg = use_signal(|| Option::<String>::None);

    let parse_uid = props.active_user_id.clone();
    let parse_block_id = props.target_block_id.clone();

    let parse_action = move |_| {
        let text = raw_csv_text.read().clone();
        if text.trim().is_empty() {
            error_msg.set(Some(
                "Please paste or upload non-empty CSV text".to_string(),
            ));
            return;
        }
        is_parsing.set(true);
        error_msg.set(None);
        let req_uid = parse_uid.clone();
        let block_id = parse_block_id.clone();

        spawn(async move {
            match parse_and_preview_csv(req_uid, text, Some(block_id)).await {
                Ok(preview) => {
                    let map: HashMap<String, String> =
                        serde_json::from_str(&preview.suggested_mappings_json).unwrap_or_default();
                    mappings_state.set(map);
                    preview_state.set(Some(preview));
                }
                Err(e) => {
                    error_msg.set(Some(format!("CSV parsing error: {}", e)));
                }
            }
            is_parsing.set(false);
        });
    };

    let exec_uid = props.active_user_id.clone();
    let exec_ws_id = props.workspace_id.clone();
    let exec_block_id = props.target_block_id.clone();
    let exec_cb = props.onimportcomplete.clone();

    let execute_action = move |_| {
        let text = raw_csv_text.read().clone();
        let map_json =
            serde_json::to_string(&*mappings_state.read()).unwrap_or_else(|_| "{}".to_string());
        is_executing.set(true);
        error_msg.set(None);
        let req_uid = exec_uid.clone();
        let ws_id = exec_ws_id.clone();
        let block_id = exec_block_id.clone();
        let oncomplete_cb = exec_cb.clone();

        spawn(async move {
            match execute_csv_import(req_uid, ws_id, block_id, map_json, text).await {
                Ok(res) => {
                    if res.failed_count > 0 {
                        error_msg.set(Some(format!(
                            "Imported {} records with {} errors",
                            res.imported_count, res.failed_count
                        )));
                    } else {
                        success_msg.set(Some(format!(
                            "Successfully imported {} records into local libSQL database!",
                            res.imported_count
                        )));
                    }
                    oncomplete_cb.call(res.imported_count);
                }
                Err(e) => {
                    error_msg.set(Some(format!("Import execution failed: {}", e)));
                }
            }
            is_executing.set(false);
        });
    };

    let preview_rows: Vec<serde_json::Value> = preview_state
        .read()
        .as_ref()
        .map(|p| serde_json::from_str(&p.preview_rows_json).unwrap_or_default())
        .unwrap_or_default();

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 sm:p-6",
            div { class: "w-full max-w-4xl max-h-[90vh] rounded-3xl border border-border bg-card p-6 shadow-2xl flex flex-col gap-5 overflow-hidden",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-sm",
                            LucideIcon { name: "file-spreadsheet", class: "h-6 w-6" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                "Intelligent CSV Import Wizard"
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-primary/10 text-primary border border-primary/20", "{props.block_name}" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Auto-detect delimiters, map column headers to database fields, and import offline in 1 click"
                            }
                        }
                    }

                    button {
                        class: "p-2 rounded-xl border border-border bg-secondary text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                        onclick: move |_| props.onclose.call(()),
                        LucideIcon { name: "x", class: "h-5 w-5" }
                    }
                }

                if let Some(ref err) = *error_msg.read() {
                    div { class: "p-3.5 rounded-xl border border-destructive/30 bg-destructive/10 text-destructive text-xs font-semibold flex items-center gap-2",
                        LucideIcon { name: "alert-triangle", class: "h-4 w-4 shrink-0" }
                        "{err}"
                    }
                }

                if let Some(ref msg) = *success_msg.read() {
                    div { class: "p-3.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-600 text-xs font-semibold flex items-center gap-2",
                        LucideIcon { name: "check-circle", class: "h-4 w-4 shrink-0" }
                        "{msg}"
                    }
                }

                // Main Content Body
                div { class: "flex-1 overflow-y-auto space-y-5 pr-1",
                    if preview_state.read().is_none() {
                        // Input Phase: Paste or upload CSV
                        div { class: "space-y-3",
                            label { class: "text-xs font-bold text-foreground block", "Paste Raw CSV Data or Drop Spreadsheet Text:" }
                            textarea {
                                class: "w-full h-56 rounded-2xl border border-border bg-background p-4 text-xs font-mono text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/40 transition-all resize-none",
                                placeholder: "client_name,priority,service_date\nAcme Corp,Urgent,2026-08-05\nNordic Logistics,Normal,2026-08-06",
                                value: "{raw_csv_text}",
                                oninput: move |e| raw_csv_text.set(e.value())
                            }

                            div { class: "flex justify-end",
                                Button {
                                    class: "text-xs h-10 px-5 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer",
                                    disabled: is_parsing,
                                    onclick: parse_action,
                                    if *is_parsing.read() {
                                        LucideIcon { name: "refresh-cw", class: "h-4 w-4 animate-spin mr-1.5" }
                                        "Detecting Schema..."
                                    } else {
                                        LucideIcon { name: "sparkles", class: "h-4 w-4 mr-1.5" }
                                        "Analyze & Auto-Map CSV"
                                    }
                                }
                            }
                        }
                    } else if let Some(prev) = preview_state.read().as_ref() {
                        div { class: "space-y-5",
                            // Summary Cards
                            div { class: "grid grid-cols-3 gap-3",
                                div { class: "p-3 rounded-xl border border-border bg-secondary/40 flex items-center justify-between",
                                    span { class: "text-xs text-muted-foreground font-medium", "Detected Delimiter" }
                                    span { class: "text-xs font-mono font-bold px-2 py-0.5 rounded bg-primary/10 text-primary border border-primary/20", "\"{prev.delimiter}\"" }
                                }
                                div { class: "p-3 rounded-xl border border-border bg-secondary/40 flex items-center justify-between",
                                    span { class: "text-xs text-muted-foreground font-medium", "Total Rows" }
                                    span { class: "text-xs font-bold text-foreground", "{prev.total_rows}" }
                                }
                                div { class: "p-3 rounded-xl border border-border bg-secondary/40 flex items-center justify-between",
                                    span { class: "text-xs text-muted-foreground font-medium", "Header Columns" }
                                    span { class: "text-xs font-bold text-foreground", "{prev.headers.len()}" }
                                }
                            }

                            // Column Mappings UI
                            div { class: "space-y-2.5",
                                h3 { class: "text-xs font-bold text-foreground m-0 flex items-center gap-1.5",
                                    LucideIcon { name: "sliders", class: "h-4 w-4 text-primary" }
                                    "Column Auto-Mapping Verification"
                                }
                                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                                    for header in prev.headers.iter() {
                                        {
                                            let h_name = header.clone();
                                            let mapped_val = mappings_state.read().get(&h_name).cloned().unwrap_or_default();
                                            rsx! {
                                                div { key: "{header}", class: "p-3 rounded-xl border border-border bg-background flex items-center justify-between gap-3",
                                                    span { class: "text-xs font-semibold text-foreground truncate max-w-[140px]", "{header}" }
                                                    div { class: "flex items-center gap-1.5 text-muted-foreground",
                                                        LucideIcon { name: "arrow-right", class: "h-3.5 w-3.5 shrink-0" }
                                                        input {
                                                            class: "h-7 px-2 text-xs rounded-lg border border-border bg-secondary text-foreground focus:outline-none focus:ring-1 focus:ring-primary w-32",
                                                            value: "{mapped_val}",
                                                            oninput: {
                                                                let h_name = h_name.clone();
                                                                move |e| {
                                                                    mappings_state.write().insert(h_name.clone(), e.value());
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

                            // Data Table Preview
                            div { class: "space-y-2 pt-2",
                                h3 { class: "text-xs font-bold text-foreground m-0 flex items-center gap-1.5",
                                    LucideIcon { name: "table", class: "h-4 w-4 text-primary" }
                                    "Sample Data Preview (First 5 Rows)"
                                }
                                div { class: "overflow-x-auto rounded-xl border border-border bg-background",
                                    table { class: "w-full text-left text-xs border-collapse",
                                        thead { class: "bg-secondary/60 text-muted-foreground font-semibold border-b border-border",
                                            tr {
                                                for h in prev.headers.iter() {
                                                    th { class: "p-2.5 whitespace-nowrap", "{h}" }
                                                }
                                            }
                                        }
                                        tbody { class: "divide-y divide-border/60 text-foreground",
                                            for (r_idx, row) in preview_rows.iter().enumerate() {
                                                tr { key: "{r_idx}", class: "hover:bg-muted/30 transition-colors",
                                                    for h in prev.headers.iter() {
                                                        td { class: "p-2.5 whitespace-nowrap truncate max-w-[160px]",
                                                            "{row.get(h).and_then(|v| v.as_str()).unwrap_or(\"\")}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Execution Footer
                            div { class: "flex items-center justify-between pt-4 border-t border-border/40",
                                Button {
                                    class: "text-xs h-9 px-3.5 rounded-xl border border-border bg-secondary text-secondary-foreground hover:bg-secondary/80 cursor-pointer",
                                    onclick: move |_| preview_state.set(None),
                                    LucideIcon { name: "arrow-left", class: "h-3.5 w-3.5 mr-1" }
                                    "Back to Input"
                                }

                                Button {
                                    class: "text-xs h-10 px-6 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer",
                                    disabled: is_executing,
                                    onclick: execute_action,
                                    if *is_executing.read() {
                                        LucideIcon { name: "refresh-cw", class: "h-4 w-4 animate-spin mr-1.5" }
                                        "Importing Records..."
                                    } else {
                                        LucideIcon { name: "check-circle", class: "h-4 w-4 mr-1.5" }
                                        "Confirm & Import {prev.total_rows} Records"
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
