use crate::components;
use crate::locales::t;
use crate::utils::use_action_runner;
use dioxus::prelude::*;
use yntra_core::{
    CalendarIntegration, CalendarSyncResult, DataImportExecutionResult, DataImportPreviewResult,
    DataImportRecord, WebhookDeliveryLog, WebhookEndpoint, Workspace, WorkspaceUser,
    delete_calendar_integration, delete_webhook_endpoint, execute_data_import,
    get_calendar_integrations, get_data_imports, get_webhook_delivery_logs, get_webhook_endpoints,
    preview_data_import, reset_webhook_circuit_breaker, retry_webhook_delivery,
    save_calendar_integration, save_webhook_endpoint, trigger_calendar_sync,
    trigger_webhook_test_event,
};

#[component]
pub fn EcosystemIntegrationsView(
    active_user: WorkspaceUser,
    workspace: Workspace,
    locale: String,
) -> Element {
    let runner = use_action_runner();
    let mut active_tab = use_signal(|| "onboarding".to_string());
    let mut db_trigger = use_signal(|| 0u32);

    // Data Onboarding State
    let mut import_entity_type = use_signal(|| "todos".to_string());
    let mut import_file_name = use_signal(|| "legacy_data_import.csv".to_string());
    let mut import_raw_content = use_signal(|| {
        "text,completed\nUpgrade server database to WAL mode,true\nMigrate legacy client records,false".to_string()
    });
    let mut import_preview = use_signal(|| None::<DataImportPreviewResult>);
    let mut import_result = use_signal(|| None::<DataImportExecutionResult>);
    let mut import_history = use_signal(Vec::<DataImportRecord>::new);
    let mut is_importing = use_signal(|| false);

    // Calendar Sync State
    let mut calendar_integrations = use_signal(Vec::<CalendarIntegration>::new);
    let mut calendar_sync_result = use_signal(|| None::<CalendarSyncResult>);
    let mut is_syncing_calendar = use_signal(|| false);
    let mut show_cal_modal = use_signal(|| false);
    let mut cal_provider = use_signal(|| "google".to_string());
    let mut cal_email = use_signal(|| "user@gmail.com".to_string());
    let mut cal_direction = use_signal(|| "two_way".to_string());

    // Webhooks State
    let mut webhook_endpoints = use_signal(Vec::<WebhookEndpoint>::new);
    let mut webhook_logs = use_signal(Vec::<WebhookDeliveryLog>::new);
    let mut show_webhook_modal = use_signal(|| false);
    let mut wh_name = use_signal(|| "Zapier Lead Sync".to_string());
    let mut wh_url = use_signal(|| "https://hooks.zapier.com/hooks/catch/998877".to_string());
    let mut wh_events = use_signal(|| "client.created,schedule.updated,todo.completed".to_string());

    // Load initial data
    let user_id = active_user.id.clone();
    let ws_id = workspace.id.clone();

    use_effect(move || {
        let _trig = db_trigger.read();
        let u_id = user_id.clone();
        let w_id = ws_id.clone();

        spawn(async move {
            if let Ok(imports) = get_data_imports(u_id.clone(), w_id.clone()).await {
                import_history.set(imports);
            }
            if let Ok(cals) = get_calendar_integrations(u_id.clone(), w_id.clone()).await {
                calendar_integrations.set(cals);
            }
            if let Ok(eps) = get_webhook_endpoints(u_id.clone(), w_id.clone()).await {
                webhook_endpoints.set(eps);
            }
            if let Ok(logs) = get_webhook_delivery_logs(u_id.clone(), w_id.clone(), None).await {
                webhook_logs.set(logs);
            }
        });
    });

    let tab_items = vec![
        ("onboarding", "1-Click CSV/Excel Importer", "download-cloud"),
        ("calendar", "2-Way Calendar Sync", "calendar"),
        ("webhooks", "Webhooks (Zapier & Make)", "webhook"),
    ];

    rsx! {
        div { class: "mx-auto w-full max-w-6xl space-y-8 p-4 md:p-8",
            // Page Header
            div { class: "flex justify-between items-center pb-6 border-b border-border",
                div { class: "flex items-center gap-4",
                    div { class: "flex h-14 w-14 items-center justify-center rounded-2xl border border-primary/20 bg-primary/10 text-primary shadow-sm",
                        components::LucideIcon { name: "plug", class: "h-7 w-7" }
                    }
                    div {
                        h1 { class: "text-3xl font-bold tracking-tight text-foreground m-0",
                            "{t(\"integrations-header-title\", &locale)}"
                        }
                        p { class: "text-sm text-muted-foreground mt-1 m-0",
                            "{t(\"integrations-header-desc\", &locale)}"
                        }
                    }
                }
            }

            // Tab Navigation
            div { class: "flex items-center gap-2 p-1.5 rounded-xl bg-muted/40 border border-border/40 max-w-max",
                for (val , label , icon) in tab_items {
                    {
                        let is_active = active_tab.read().as_str() == val;
                        let val_str = val.to_string();
                        rsx! {
                            button {
                                key: "{val}",
                                class: format!(
                                    "flex items-center gap-2.5 px-4 py-2 text-xs font-semibold rounded-lg transition-all border-0 cursor-pointer {}",
                                    if is_active { "bg-primary text-primary-foreground shadow-sm" } else { "bg-transparent text-muted-foreground hover:text-foreground hover:bg-muted/60" }
                                ),
                                onclick: move |_| active_tab.set(val_str.clone()),
                                components::LucideIcon { name: icon, class: "h-4 w-4" }
                                span { "{label}" }
                            }
                        }
                    }
                }
            }

            // TAB 1: 1-Click CSV/Excel Data Importer
            if active_tab.read().as_str() == "onboarding" {
                div { class: "space-y-6",
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-6",
                        div { class: "flex items-center justify-between border-b border-border/60 pb-4",
                            div {
                                h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "file-spread-sheet", class: "h-5 w-5 text-primary" }
                                    "1-Click Legacy Data Importer"
                                }
                                p { class: "text-xs text-muted-foreground mt-1 m-0",
                                    "Migrate legacy records (Clients, Users, Scheduling, Notes, Todos, Jobs) from CSV or Excel files."
                                }
                            }
                            button {
                                class: "flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-lg border border-border bg-muted/50 hover:bg-muted text-foreground transition-all cursor-pointer",
                                onclick: move |_| {
                                    import_raw_content.set("text,completed\nSample todo task item 1,true\nSample todo task item 2,false".to_string());
                                },
                                components::LucideIcon { name: "file-text", class: "h-3.5 w-3.5 text-primary" }
                                "Load Sample CSV"
                            }
                        }

                        div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                            div { class: "space-y-2",
                                label { class: "text-xs font-semibold text-foreground", "Target Module Entity" }
                                select {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none focus:ring-2 focus:ring-primary/20",
                                    value: "{import_entity_type}",
                                    onchange: move |e| import_entity_type.set(e.value()),
                                    option { value: "todos", "Todos & Quick Tasks" }
                                    option { value: "clients", "Clients & Care Records" }
                                    option { value: "directory", "User Directory & Team" }
                                    option { value: "events", "Calendar Events & Shifts" }
                                    option { value: "notes", "Daily Notes & Journal Entries" }
                                    option { value: "jobs", "Service Tickets & Jobs" }
                                }
                            }

                            div { class: "space-y-2 md:col-span-2",
                                label { class: "text-xs font-semibold text-foreground", "File Name" }
                                input {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none focus:ring-2 focus:ring-primary/20",
                                    value: "{import_file_name}",
                                    oninput: move |e| import_file_name.set(e.value()),
                                    placeholder: "legacy_export.csv",
                                }
                            }
                        }

                        div { class: "space-y-2",
                            label { class: "text-xs font-semibold text-foreground flex items-center justify-between",
                                span { "CSV / Tab-Separated Data Text" }
                                span { class: "text-xs text-muted-foreground font-normal", "Supports headers, commas, tabs, and semicolons" }
                            }
                            textarea {
                                class: "w-full h-36 rounded-xl border border-border bg-background p-3 text-xs font-mono text-foreground outline-none focus:ring-2 focus:ring-primary/20 resize-none",
                                value: "{import_raw_content}",
                                oninput: move |e| import_raw_content.set(e.value()),
                                placeholder: "header1,header2,header3\nval1,val2,val3",
                            }
                        }

                        div { class: "flex items-center gap-3 pt-2",
                            button {
                                class: "flex items-center gap-2 px-4 py-2 text-xs font-semibold rounded-xl border border-border bg-secondary text-secondary-foreground hover:bg-secondary/80 transition-all cursor-pointer",
                                onclick: {
                                    let u_id = active_user.id.clone();
                                    let w_id = workspace.id.clone();
                                    let r_runner = runner.clone();
                                    move |_| {
                                        let u_id = u_id.clone();
                                        let w_id = w_id.clone();
                                        let ent = import_entity_type.read().clone();
                                        let fname = import_file_name.read().clone();
                                        let raw = import_raw_content.read().clone();

                                        r_runner.clone().run(async move {
                                            let prev = preview_data_import(u_id, w_id, ent, fname, raw).await?;
                                            import_preview.set(Some(prev));
                                            Ok(())
                                        });
                                    }
                                },
                                components::LucideIcon { name: "search", class: "h-4 w-4" }
                                "1. Preview Schema Mapping"
                            }

                            button {
                                class: "flex items-center gap-2 px-5 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer shadow-md shadow-primary/20",
                                disabled: *is_importing.read(),
                                onclick: {
                                    let u_id = active_user.id.clone();
                                    let w_id = workspace.id.clone();
                                    let r_runner = runner.clone();
                                    move |_| {
                                        let u_id = u_id.clone();
                                        let w_id = w_id.clone();
                                        let ent = import_entity_type.read().clone();
                                        let fname = import_file_name.read().clone();
                                        let raw = import_raw_content.read().clone();
                                        is_importing.set(true);

                                        r_runner.clone().run(async move {
                                            let res = execute_data_import(u_id, w_id, ent, fname, raw).await?;
                                            import_result.set(Some(res));
                                            is_importing.set(false);
                                            db_trigger.with_mut(|v| *v += 1);
                                            Ok(())
                                        });
                                    }
                                },
                                if *is_importing.read() {
                                    components::LucideIcon { name: "refresh-cw", class: "h-4 w-4 animate-spin" }
                                    "Importing..."
                                } else {
                                    components::LucideIcon { name: "upload-cloud", class: "h-4 w-4" }
                                    "2. Execute 1-Click Import"
                                }
                            }
                        }

                        // Schema Preview Box
                        if let Some(ref prev) = *import_preview.read() {
                            div { class: "rounded-xl border border-primary/20 bg-primary/5 p-4 space-y-3",
                                div { class: "flex items-center justify-between text-xs font-semibold text-primary",
                                    span { "Preview: {prev.total_rows} Total Rows ({prev.valid_rows} Valid, {prev.invalid_rows} Invalid)" }
                                    span { class: "font-mono text-muted-foreground", "Delimiter: '{prev.detected_delimiter}'" }
                                }
                                div { class: "flex flex-wrap gap-1.5 text-xs",
                                    span { class: "text-muted-foreground font-medium mr-1", "Detected Columns:" }
                                    for col in prev.columns_detected.iter() {
                                        span { key: "{col}", class: "px-2 py-0.5 rounded-md bg-background border border-border text-foreground font-mono text-[11px]", "{col}" }
                                    }
                                }
                            }
                        }

                        // Execution Result Summary Alert
                        if let Some(ref res) = *import_result.read() {
                            div { class: "rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-4 space-y-2 text-xs",
                                div { class: "flex items-center gap-2 font-bold text-emerald-600 dark:text-emerald-400 text-sm",
                                    components::LucideIcon { name: "check-circle", class: "h-5 w-5" }
                                    "Data Onboarding Completed Successfully!"
                                }
                                p { class: "text-muted-foreground m-0",
                                    "Imported {res.records_imported} of {res.records_total} records into {res.entity_type} table. Status: {res.status}."
                                }
                            }
                        }
                    }

                    // Historical Import Batches Table
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                        h3 { class: "text-md font-bold text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "history", class: "h-4 w-4 text-primary" }
                            "Historical Migration Batches"
                        }

                        if import_history.read().is_empty() {
                            p { class: "text-xs text-muted-foreground italic m-0", "No previous data imports recorded in this workspace." }
                        } else {
                            div { class: "overflow-x-auto",
                                table { class: "w-full text-left text-xs border-collapse",
                                    thead { class: "border-b border-border bg-muted/30 text-muted-foreground uppercase text-[10px] tracking-wider",
                                        tr {
                                            th { class: "p-3 font-semibold", "File Name" }
                                            th { class: "p-3 font-semibold", "Target Entity" }
                                            th { class: "p-3 font-semibold", "Format" }
                                            th { class: "p-3 font-semibold", "Imported / Total" }
                                            th { class: "p-3 font-semibold", "Status" }
                                        }
                                    }
                                    tbody { class: "divide-y divide-border/60",
                                        for item in import_history.read().iter() {
                                            tr { key: "{item.id}", class: "hover:bg-muted/20 transition-colors",
                                                td { class: "p-3 font-medium text-foreground font-mono", "{item.file_name}" }
                                                td { class: "p-3 text-muted-foreground uppercase font-bold text-[10px]", "{item.entity_type}" }
                                                td { class: "p-3 text-muted-foreground uppercase", "{item.file_format}" }
                                                td { class: "p-3 text-foreground font-semibold", "{item.records_imported} / {item.records_total}" }
                                                td { class: "p-3",
                                                    span { class: "inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                                        "{item.status}"
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

            // TAB 2: 2-Way Calendar Sync
            if active_tab.read().as_str() == "calendar" {
                div { class: "space-y-6",
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                        // Google Calendar Card
                        div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-3",
                                    div { class: "flex h-10 w-10 items-center justify-center rounded-xl bg-blue-500/10 text-blue-500 border border-blue-500/20",
                                        components::LucideIcon { name: "calendar", class: "h-5 w-5" }
                                    }
                                    div {
                                        h3 { class: "text-base font-bold text-foreground m-0", "Google Calendar" }
                                        p { class: "text-xs text-muted-foreground m-0", "2-Way OAuth Calendar Sync" }
                                    }
                                }
                                span { class: "px-2.5 py-1 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                    "Ready"
                                }
                            }
                            p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                                "Synchronize shifts, scheduling appointments, and events between Yntra local database and Google Calendar automatically."
                            }
                            button {
                                class: "w-full flex items-center justify-center gap-2 px-4 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer shadow-sm",
                                onclick: move |_| {
                                    cal_provider.set("google".to_string());
                                    show_cal_modal.set(true);
                                },
                                components::LucideIcon { name: "settings", class: "h-4 w-4" }
                                "Configure Google Sync"
                            }
                        }

                        // Outlook Calendar Card
                        div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-3",
                                    div { class: "flex h-10 w-10 items-center justify-center rounded-xl bg-sky-500/10 text-sky-500 border border-sky-500/20",
                                        components::LucideIcon { name: "mail", class: "h-5 w-5" }
                                    }
                                    div {
                                        h3 { class: "text-base font-bold text-foreground m-0", "Microsoft Outlook" }
                                        p { class: "text-xs text-muted-foreground m-0", "Microsoft Graph Calendar API" }
                                    }
                                }
                                span { class: "px-2.5 py-1 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                    "Ready"
                                }
                            }
                            p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                                "Sync enterprise schedules, client meetings, and team dispatches directly with Office 365 / Outlook calendar."
                            }
                            button {
                                class: "w-full flex items-center justify-center gap-2 px-4 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer shadow-sm",
                                onclick: move |_| {
                                    cal_provider.set("outlook".to_string());
                                    show_cal_modal.set(true);
                                },
                                components::LucideIcon { name: "settings", class: "h-4 w-4" }
                                "Configure Outlook Sync"
                            }
                        }
                    }

                    // Active Connections & Trigger Sync Table
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                        h3 { class: "text-md font-bold text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "refresh-cw", class: "h-4 w-4 text-primary" }
                            "Active Calendar Connection Feeds"
                        }

                        if calendar_integrations.read().is_empty() {
                            div { class: "rounded-xl border border-dashed border-border p-6 text-center text-xs text-muted-foreground space-y-2",
                                p { class: "m-0 font-medium", "No calendar feeds connected yet." }
                                p { class: "m-0 text-[11px]", "Click 'Configure Google Sync' or 'Configure Outlook Sync' above to initialize 2-way sync." }
                            }
                        } else {
                            div { class: "overflow-x-auto",
                                table { class: "w-full text-left text-xs border-collapse",
                                    thead { class: "border-b border-border bg-muted/30 text-muted-foreground uppercase text-[10px] tracking-wider",
                                        tr {
                                            th { class: "p-3 font-semibold", "Provider" }
                                            th { class: "p-3 font-semibold", "Account Email" }
                                            th { class: "p-3 font-semibold", "Direction" }
                                            th { class: "p-3 font-semibold", "Status" }
                                            th { class: "p-3 font-semibold text-right", "Actions" }
                                        }
                                    }
                                    tbody { class: "divide-y divide-border/60",
                                        for item in calendar_integrations.read().iter() {
                                            {
                                                let item_id = item.id.clone();
                                                rsx! {
                                                    tr { key: "{item.id}", class: "hover:bg-muted/20 transition-colors",
                                                        td { class: "p-3 font-bold text-foreground uppercase text-[11px]", "{item.provider}" }
                                                        td { class: "p-3 font-medium text-foreground", "{item.account_email}" }
                                                        td { class: "p-3 text-muted-foreground uppercase text-[10px]", "{item.sync_direction}" }
                                                        td { class: "p-3",
                                                            span { class: "inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                                                "{item.sync_status}"
                                                            }
                                                        }
                                                        td { class: "p-3 text-right space-x-2",
                                                            button {
                                                                class: "px-3 py-1 text-xs font-semibold rounded-lg bg-primary/10 text-primary hover:bg-primary/20 border border-primary/20 transition-all cursor-pointer",
                                                                disabled: *is_syncing_calendar.read(),
                                                                onclick: {
                                                                    let u_id = active_user.id.clone();
                                                                    let w_id = workspace.id.clone();
                                                                    let i_id = item_id.clone();
                                                                    let r_runner = runner.clone();
                                                                    move |_| {
                                                                        let u_id = u_id.clone();
                                                                        let w_id = w_id.clone();
                                                                        let i_id = i_id.clone();
                                                                        is_syncing_calendar.set(true);

                                                                        r_runner.clone().run(async move {
                                                                            let res = trigger_calendar_sync(u_id, w_id, i_id).await?;
                                                                            calendar_sync_result.set(Some(res));
                                                                            is_syncing_calendar.set(false);
                                                                            db_trigger.with_mut(|v| *v += 1);
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "Sync Now"
                                                            }
                                                            button {
                                                                class: "px-2.5 py-1 text-xs font-semibold rounded-lg bg-destructive/10 text-destructive hover:bg-destructive/20 border border-destructive/20 transition-all cursor-pointer",
                                                                onclick: {
                                                                    let u_id = active_user.id.clone();
                                                                    let w_id = workspace.id.clone();
                                                                    let i_id = item_id.clone();
                                                                    let r_runner = runner.clone();
                                                                    move |_| {
                                                                        let u_id = u_id.clone();
                                                                        let w_id = w_id.clone();
                                                                        let i_id = i_id.clone();

                                                                        r_runner.clone().run(async move {
                                                                            delete_calendar_integration(u_id, w_id, i_id).await?;
                                                                            db_trigger.with_mut(|v| *v += 1);
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "Disconnect"
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

                        if let Some(ref res) = *calendar_sync_result.read() {
                            div { class: "rounded-xl border border-blue-500/30 bg-blue-500/10 p-4 text-xs space-y-1",
                                div { class: "font-bold text-blue-600 dark:text-blue-400 flex items-center gap-2",
                                    components::LucideIcon { name: "check-circle", class: "h-4 w-4" }
                                    "2-Way Calendar Sync Finished ({res.provider})"
                                }
                                p { class: "text-muted-foreground m-0",
                                    "Events Pulled: {res.events_pulled} | Events Pushed: {res.events_pushed} | Conflicts Resolved: {res.conflicts_resolved}"
                                }
                            }
                        }
                    }
                }
            }

            // TAB 3: External Webhooks (Zapier & Make)
            if active_tab.read().as_str() == "webhooks" {
                div { class: "space-y-6",
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-6",
                        div { class: "flex items-center justify-between border-b border-border/60 pb-4",
                            div {
                                h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "webhook", class: "h-5 w-5 text-primary" }
                                    "External Webhooks (Zapier / Make / Custom Endpoints)"
                                }
                                p { class: "text-xs text-muted-foreground mt-1 m-0",
                                    "Broadcast entity updates in real-time with HMAC-SHA256 signatures in X-Yntra-Signature headers."
                                }
                            }
                            button {
                                class: "flex items-center gap-2 px-4 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer shadow-sm",
                                onclick: move |_| show_webhook_modal.set(true),
                                components::LucideIcon { name: "plus", class: "h-4 w-4" }
                                "Add Webhook Endpoint"
                            }
                        }

                        // Webhook Endpoints Grid
                        if webhook_endpoints.read().is_empty() {
                            div { class: "rounded-xl border border-dashed border-border p-6 text-center text-xs text-muted-foreground space-y-2",
                                p { class: "m-0 font-medium", "No webhook endpoints registered yet." }
                                p { class: "m-0 text-[11px]", "Connect your Zapier or Make catch webhooks to automate workflows on client creation or schedule updates." }
                            }
                        } else {
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                for ep in webhook_endpoints.read().iter() {
                                    {
                                        let ep_id = ep.id.clone();
                                        let ep_secret = ep.secret.clone();
                                        rsx! {
                                            div { key: "{ep.id}", class: "rounded-xl border border-border bg-background p-4 space-y-3 shadow-xs",
                                                div { class: "flex items-start justify-between",
                                                    div {
                                                        h4 { class: "text-sm font-bold text-foreground m-0", "{ep.name}" }
                                                        p { class: "text-[11px] font-mono text-muted-foreground m-0 mt-0.5 truncate max-w-[240px]", "{ep.target_url}" }
                                                    }
                                                    div { class: "flex items-center gap-1.5",
                                                        span { class: format!(
                                                            "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase border {}",
                                                            if ep.circuit_state == "open" { "bg-destructive/10 text-destructive border-destructive/20" } else { "bg-emerald-500/10 text-emerald-600 border-emerald-500/20" }
                                                        ),
                                                            "Circuit: {ep.circuit_state}"
                                                        }
                                                        span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-primary/10 text-primary border border-primary/20",
                                                            if ep.is_active { "Active" } else { "Inactive" }
                                                        }
                                                    }
                                                }

                                                div { class: "flex flex-wrap gap-1 text-[10px]",
                                                    for ev in ep.events.iter() {
                                                        span { key: "{ev}", class: "px-2 py-0.5 rounded-md bg-muted text-muted-foreground font-mono", "{ev}" }
                                                    }
                                                }

                                                div { class: "flex items-center justify-between text-xs pt-2 border-t border-border/40",
                                                    span { class: "text-[10px] font-mono text-muted-foreground truncate max-w-[140px]", "Secret: {ep_secret}" }
                                                    div { class: "flex items-center gap-2",
                                                        if ep.circuit_state == "open" {
                                                            button {
                                                                class: "px-2.5 py-1 text-[11px] font-semibold rounded-lg bg-amber-500/10 text-amber-600 hover:bg-amber-500/20 border border-amber-500/20 transition-all cursor-pointer",
                                                                onclick: {
                                                                    let u_id = active_user.id.clone();
                                                                    let w_id = workspace.id.clone();
                                                                    let e_id = ep_id.clone();
                                                                    let r_runner = runner.clone();
                                                                    move |_| {
                                                                        let u_id = u_id.clone();
                                                                        let w_id = w_id.clone();
                                                                        let e_id = e_id.clone();

                                                                        r_runner.clone().run(async move {
                                                                            reset_webhook_circuit_breaker(u_id, w_id, e_id).await?;
                                                                            db_trigger.with_mut(|v| *v += 1);
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "Reset Circuit"
                                                            }
                                                        }
                                                        button {
                                                            class: "px-2.5 py-1 text-[11px] font-semibold rounded-lg bg-primary/10 text-primary hover:bg-primary/20 border border-primary/20 transition-all cursor-pointer",
                                                            onclick: {
                                                                let u_id = active_user.id.clone();
                                                                let w_id = workspace.id.clone();
                                                                let e_id = ep_id.clone();
                                                                let r_runner = runner.clone();
                                                                move |_| {
                                                                    let u_id = u_id.clone();
                                                                    let w_id = w_id.clone();
                                                                    let e_id = e_id.clone();

                                                                    r_runner.clone().run(async move {
                                                                        trigger_webhook_test_event(u_id, w_id, e_id).await?;
                                                                        db_trigger.with_mut(|v| *v += 1);
                                                                        Ok(())
                                                                    });
                                                                }
                                                            },
                                                            "Test Dispatch"
                                                        }
                                                        button {
                                                            class: "px-2 py-1 text-[11px] font-semibold rounded-lg bg-destructive/10 text-destructive hover:bg-destructive/20 border border-destructive/20 transition-all cursor-pointer",
                                                            onclick: {
                                                                let u_id = active_user.id.clone();
                                                                let w_id = workspace.id.clone();
                                                                let e_id = ep_id.clone();
                                                                let r_runner = runner.clone();
                                                                move |_| {
                                                                    let u_id = u_id.clone();
                                                                    let w_id = w_id.clone();
                                                                    let e_id = e_id.clone();

                                                                    r_runner.clone().run(async move {
                                                                        delete_webhook_endpoint(u_id, w_id, e_id).await?;
                                                                        db_trigger.with_mut(|v| *v += 1);
                                                                        Ok(())
                                                                    });
                                                                }
                                                            },
                                                            "Delete"
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

                    // Delivery Logs Inspector
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                        h3 { class: "text-md font-bold text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "activity", class: "h-4 w-4 text-primary" }
                            "Recent Webhook Delivery History"
                        }

                        if webhook_logs.read().is_empty() {
                            p { class: "text-xs text-muted-foreground italic m-0", "No webhook delivery attempts recorded yet." }
                        } else {
                            div { class: "overflow-x-auto",
                                table { class: "w-full text-left text-xs border-collapse",
                                    thead { class: "border-b border-border bg-muted/30 text-muted-foreground uppercase text-[10px] tracking-wider",
                                        tr {
                                            th { class: "p-3 font-semibold", "Event Type" }
                                            th { class: "p-3 font-semibold", "HTTP Response" }
                                            th { class: "p-3 font-semibold", "Attempts" }
                                            th { class: "p-3 font-semibold", "Status" }
                                            th { class: "p-3 font-semibold text-right", "Action" }
                                        }
                                    }
                                    tbody { class: "divide-y divide-border/60",
                                        for log in webhook_logs.read().iter() {
                                            {
                                                let log_id = log.id.clone();
                                                rsx! {
                                                    tr { key: "{log.id}", class: "hover:bg-muted/20 transition-colors",
                                                        td { class: "p-3 font-bold font-mono text-foreground text-[11px]", "{log.event_type}" }
                                                        td { class: "p-3 font-mono text-emerald-600 font-bold", "{log.response_code} OK" }
                                                        td { class: "p-3 text-muted-foreground font-semibold", "{log.attempt_count}" }
                                                        td { class: "p-3",
                                                            span { class: "inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                                                "{log.status}"
                                                            }
                                                        }
                                                        td { class: "p-3 text-right",
                                                            button {
                                                                class: "px-2.5 py-1 text-xs font-semibold rounded-lg bg-secondary text-secondary-foreground hover:bg-secondary/80 border border-border transition-all cursor-pointer",
                                                                onclick: {
                                                                    let u_id = active_user.id.clone();
                                                                    let w_id = workspace.id.clone();
                                                                    let l_id = log_id.clone();
                                                                    let r_runner = runner.clone();
                                                                    move |_| {
                                                                        let u_id = u_id.clone();
                                                                        let w_id = w_id.clone();
                                                                        let l_id = l_id.clone();

                                                                        r_runner.clone().run(async move {
                                                                            retry_webhook_delivery(u_id, w_id, l_id).await?;
                                                                            db_trigger.with_mut(|v| *v += 1);
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "Retry Delivery"
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

            // MODAL 1: Calendar Settings Modal
            if *show_cal_modal.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4",
                    div { class: "w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-4",
                        h3 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "calendar", class: "h-5 w-5 text-primary" }
                            "Configure Calendar Connection"
                        }

                        div { class: "space-y-3 text-xs",
                            div { class: "space-y-1",
                                label { class: "font-semibold text-foreground", "Calendar Account Email" }
                                input {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none",
                                    value: "{cal_email}",
                                    oninput: move |e| cal_email.set(e.value()),
                                    placeholder: "account@organization.com",
                                }
                            }

                            div { class: "space-y-1",
                                label { class: "font-semibold text-foreground", "Sync Direction" }
                                select {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none",
                                    value: "{cal_direction}",
                                    onchange: move |e| cal_direction.set(e.value()),
                                    option { value: "two_way", "2-Way Bi-directional Sync" }
                                    option { value: "pull_only", "Pull Only (Remote -> Yntra)" }
                                    option { value: "push_only", "Push Only (Yntra -> Remote)" }
                                }
                            }
                        }

                        div { class: "flex items-center justify-end gap-3 pt-4 border-t border-border",
                            button {
                                class: "px-4 py-2 text-xs font-semibold rounded-xl border border-border bg-transparent text-muted-foreground hover:bg-muted cursor-pointer",
                                onclick: move |_| show_cal_modal.set(false),
                                "Cancel"
                            }
                            button {
                                class: "px-5 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 cursor-pointer shadow-sm",
                                onclick: {
                                    let u_id = active_user.id.clone();
                                    let w_id = workspace.id.clone();
                                    let r_runner = runner.clone();
                                    move |_| {
                                        let u_id = u_id.clone();
                                        let w_id = w_id.clone();
                                        let prov = cal_provider.read().clone();
                                        let email = cal_email.read().clone();
                                        let dir = cal_direction.read().clone();

                                        r_runner.clone().run(async move {
                                            let item = CalendarIntegration {
                                                id: "".to_string(),
                                                workspace_id: w_id,
                                                provider: prov,
                                                account_email: email,
                                                access_token: Some("oauth_access_token".to_string()),
                                                refresh_token: Some("oauth_refresh_token".to_string()),
                                                token_expires_at: 1800000000,
                                                sync_direction: dir,
                                                auto_sync_enabled: true,
                                                last_synced_at: 0,
                                                sync_status: "idle".to_string(),
                                                error_message: None,
                                                sync_token: None,
                                                created_at: 0,
                                                updated_at: 0,
                                            };
                                            save_calendar_integration(u_id, item).await?;
                                            show_cal_modal.set(false);
                                            db_trigger.with_mut(|v| *v += 1);
                                            Ok(())
                                        });
                                    }
                                },
                                "Save Connection"
                            }
                        }
                    }
                }
            }

            // MODAL 2: Add Webhook Modal
            if *show_webhook_modal.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4",
                    div { class: "w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-4",
                        h3 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "webhook", class: "h-5 w-5 text-primary" }
                            "Add Webhook Endpoint"
                        }

                        div { class: "space-y-3 text-xs",
                            div { class: "space-y-1",
                                label { class: "font-semibold text-foreground", "Webhook Name" }
                                input {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none",
                                    value: "{wh_name}",
                                    oninput: move |e| wh_name.set(e.value()),
                                    placeholder: "Zapier Lead Sync",
                                }
                            }

                            div { class: "space-y-1",
                                label { class: "font-semibold text-foreground", "Target URL" }
                                input {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none",
                                    value: "{wh_url}",
                                    oninput: move |e| wh_url.set(e.value()),
                                    placeholder: "https://hooks.zapier.com/...",
                                }
                            }

                            div { class: "space-y-1",
                                label { class: "font-semibold text-foreground", "Subscribed Events (Comma-separated)" }
                                input {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none",
                                    value: "{wh_events}",
                                    oninput: move |e| wh_events.set(e.value()),
                                    placeholder: "client.created,schedule.updated",
                                }
                            }
                        }

                        div { class: "flex items-center justify-end gap-3 pt-4 border-t border-border",
                            button {
                                class: "px-4 py-2 text-xs font-semibold rounded-xl border border-border bg-transparent text-muted-foreground hover:bg-muted cursor-pointer",
                                onclick: move |_| show_webhook_modal.set(false),
                                "Cancel"
                            }
                            button {
                                class: "px-5 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 cursor-pointer shadow-sm",
                                onclick: {
                                    let u_id = active_user.id.clone();
                                    let w_id = workspace.id.clone();
                                    let r_runner = runner.clone();
                                    move |_| {
                                        let u_id = u_id.clone();
                                        let w_id = w_id.clone();
                                        let name = wh_name.read().clone();
                                        let url = wh_url.read().clone();
                                        let evs_str = wh_events.read().clone();
                                        let evs_vec: Vec<String> = evs_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

                                        r_runner.clone().run(async move {
                                            let ep = WebhookEndpoint {
                                                id: "".to_string(),
                                                workspace_id: w_id,
                                                name,
                                                target_url: url,
                                                secret: "".to_string(),
                                                events: evs_vec,
                                                is_active: true,
                                                consecutive_failures: 0,
                                                circuit_state: "closed".to_string(),
                                                created_at: 0,
                                                updated_at: 0,
                                            };
                                            save_webhook_endpoint(u_id, ep).await?;
                                            show_webhook_modal.set(false);
                                            db_trigger.with_mut(|v| *v += 1);
                                            Ok(())
                                        });
                                    }
                                },
                                "Save Webhook"
                            }
                        }
                    }
                }
            }
        }
    }
}
