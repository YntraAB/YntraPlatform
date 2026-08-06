use crate::components;
use crate::locales::t;
use crate::utils::use_action_runner;
use dioxus::prelude::*;
use yntra_core::{
    CalendarIntegration, CalendarSyncResult, DataImportExecutionResult, DataImportPreviewResult,
    DataImportRecord, FdaDualSignatureRecord, FhirResourceMappingRecord, Hl7MessageRecord,
    MllpListenerConfig, NcpdpPrescriptionRecord, WebhookDeliveryLog, WebhookEndpoint, Workspace,
    WorkspaceUser, cancel_ncpdp_prescription, create_ncpdp_new_rx_prescription,
    delete_calendar_integration, delete_mllp_listener, delete_webhook_endpoint,
    execute_data_import, execute_fda_part11_dual_signature, export_condition_to_fhir_r4,
    export_encounter_to_fhir_r4, export_observation_to_fhir_r4, export_patient_to_fhir_r4,
    export_workspace_fhir_bundle, get_calendar_integrations, get_data_imports,
    get_fda_part11_signatures, get_fhir_resource_mappings, get_hl7_messages, get_mllp_listeners,
    get_ncpdp_prescriptions, get_webhook_delivery_logs, get_webhook_endpoints,
    import_fhir_r4_resource, is_server_gateway_compiled, parse_and_import_inbound_ncpdp_xml, preview_data_import,
    process_raw_hl7_v2_message, reset_webhook_circuit_breaker, retry_webhook_delivery,
    save_calendar_integration, save_mllp_listener, save_webhook_endpoint, start_mllp_listener,
    stop_mllp_listener, transmit_ncpdp_script_to_surescripts, trigger_calendar_sync,
    trigger_webhook_test_event, verify_fda_part11_dual_signature,
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

    // MLLP & HL7 v2 State
    let mut mllp_listeners = use_signal(Vec::<MllpListenerConfig>::new);
    let mut hl7_messages = use_signal(Vec::<Hl7MessageRecord>::new);
    let mut show_mllp_modal = use_signal(|| false);
    let mut mllp_name = use_signal(|| "Hospital ADT MLLP Listener".to_string());
    let mut mllp_port = use_signal(|| 2575u16);
    let mut selected_hl7_msg = use_signal(|| None::<Hl7MessageRecord>);
    let mut test_er7_input = use_signal(|| {
        "MSH|^~\\&|EPIC_EHR|GENERAL_HOSP|YNTRA_CORE|YNTRA_FAC|20260806140000||ADT^A01|CTRL1001|P|2.3\rPID|1||MRN778899^^^HOSP||SMITH^JANE^ELIZABETH||19920515|F|||456 PARK AVE^^SAN FRANCISCO^CA^94102||555-0199||\rPV1|1|I|ICU^BED-04^ROOM-101||||1234^STEVENS^MARK||||||||||||V998877|||||||||||||||||||||||||20260806140000|".to_string()
    });

    // FHIR R4 Engine State
    let mut fhir_mappings = use_signal(Vec::<FhirResourceMappingRecord>::new);
    let mut selected_fhir_type = use_signal(|| "Patient".to_string());
    let mut exported_fhir_json = use_signal(String::new);
    let mut import_fhir_input = use_signal(|| {
        r#"{
  "resourceType": "Patient",
  "id": "ext-pat-100",
  "active": true,
  "name": [{ "text": "ALEXANDER GRAHAM", "family": "GRAHAM", "given": ["ALEXANDER"] }],
  "identifier": [{ "system": "MRN", "value": "MRN-10099" }]
}"#.to_string()
    });
    let mut import_fhir_status = use_signal(|| None::<(bool, String)>);

    // NCPDP SCRIPT e-Prescribing State
    let mut ncpdp_prescriptions = use_signal(Vec::<NcpdpPrescriptionRecord>::new);
    let mut ncpdp_drug_name = use_signal(|| "Amoxicillin 500mg Oral Capsule".to_string());
    let mut ncpdp_prescriber_npi = use_signal(|| "1992003004".to_string());
    let mut ncpdp_pharmacy_npi = use_signal(|| "1881002003".to_string());
    let mut ncpdp_quantity = use_signal(|| 30.0f64);
    let mut ncpdp_days_supply = use_signal(|| 10u32);
    let mut ncpdp_refills = use_signal(|| 2u32);
    let mut ncpdp_sig = use_signal(|| "Take 1 capsule by mouth three times daily for 10 days".to_string());
    let mut generated_ncpdp_xml = use_signal(String::new);
    let mut inbound_ncpdp_input = use_signal(|| {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Message version="v2017071" xmlns="http://www.ncpdp.org/schema/SCRIPT">
  <Header>
    <To Qualifier="P">1992003004</To>
    <From Qualifier="C">1881002003</From>
    <MessageID>msg-inbound-888</MessageID>
    <RelatesToMessageID>rx-orig-500</RelatesToMessageID>
  </Header>
  <Body>
    <RxRenewalRequest>
      <StoreName>Walgreens Pharmacy #2041</StoreName>
      <MedicationPrescribed>
        <DrugDescription>Metformin 500mg Oral Tablet</DrugDescription>
        <QuantityValue>90</QuantityValue>
        <DaysSupply>90</DaysSupply>
        <Refills>3</Refills>
        <Sig>Take 1 tablet twice daily with meals</Sig>
      </MedicationPrescribed>
    </RxRenewalRequest>
  </Body>
</Message>"#.to_string()
    });
    let mut ncpdp_status_alert = use_signal(|| None::<(bool, String)>);

    // FDA 21 CFR Part 11 State
    let mut fda_signatures = use_signal(Vec::<FdaDualSignatureRecord>::new);
    let mut fda_target_type = use_signal(|| "clinical_chart".to_string());
    let mut fda_target_id = use_signal(|| "chart-note-8800".to_string());
    let mut fda_primary_name = use_signal(|| "Dr. Alice Smith, MD".to_string());
    let mut fda_primary_intent = use_signal(|| "Authorship & Clinical Entry".to_string());
    let mut fda_secondary_user_id = use_signal(|| "user-2".to_string());
    let mut fda_secondary_name = use_signal(|| "Dr. Robert Vance, MD".to_string());
    let mut fda_secondary_intent = use_signal(|| "Cosigning Supervision Approval".to_string());
    let mut fda_require_dual = use_signal(|| true);
    let mut fda_status_alert = use_signal(|| None::<(bool, String)>);

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
            if let Ok(listeners) = get_mllp_listeners(u_id.clone(), w_id.clone()).await {
                mllp_listeners.set(listeners);
            }
            if let Ok(msgs) = get_hl7_messages(u_id.clone(), w_id.clone(), None, Some(50)).await {
                hl7_messages.set(msgs);
            }
            if let Ok(mappings) = get_fhir_resource_mappings(u_id.clone(), w_id.clone()).await {
                fhir_mappings.set(mappings);
            }
            if let Ok(rxs) = get_ncpdp_prescriptions(u_id.clone(), w_id.clone(), None).await {
                ncpdp_prescriptions.set(rxs);
            }
            if let Ok(sigs) = get_fda_part11_signatures(u_id.clone(), w_id.clone(), None).await {
                fda_signatures.set(sigs);
            }
        });
    });

    let tab_items = vec![
        ("onboarding", "1-Click CSV/Excel Importer", "download-cloud"),
        ("calendar", "2-Way Calendar Sync", "calendar"),
        ("webhooks", "Webhooks (Zapier & Make)", "webhook"),
        ("mllp", "HL7 v2 MLLP Feeds", "activity"),
        ("fhir", "FHIR R4 Engine", "database"),
        ("ncpdp", "NCPDP e-Prescribing", "file-text"),
        ("fda_part11", "FDA 21 CFR Part 11", "shield-check"),
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

            // TAB 4: HL7 v2 MLLP Socket Listener Feeds
            if active_tab.read().as_str() == "mllp" {
                div { class: "space-y-6",
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-6",
                        div { class: "flex items-center justify-between border-b border-border/60 pb-4",
                            div {
                                h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "activity", class: "h-5 w-5 text-primary" }
                                    "HL7 v2 MLLP Gateway Feeds (Hospital ADT / ORU / ORM)"
                                }
                                p { class: "text-xs text-muted-foreground mt-1 m-0",
                                    "Enterprise Server Gateway Minimal Lower Layer Protocol over TCP socket ingestion with pipe-delimited segment parsing & automated ACK responses."
                                }
                            }
                            if is_server_gateway_compiled() {
                                button {
                                    class: "flex items-center gap-2 px-4 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer shadow-sm",
                                    onclick: move |_| show_mllp_modal.set(true),
                                    components::LucideIcon { name: "plus", class: "h-4 w-4" }
                                    "New MLLP Listener"
                                }
                            } else {
                                span {
                                    class: "px-3 py-1.5 text-xs font-semibold rounded-xl bg-amber-500/10 text-amber-500 border border-amber-500/20 flex items-center gap-1.5",
                                    components::LucideIcon { name: "lock", class: "h-3.5 w-3.5" }
                                    "Server Gateway Build Required"
                                }
                            }
                        }

                        // Enterprise Security Governance Banner
                        div { class: "rounded-xl border border-blue-500/20 bg-blue-500/5 p-4 space-y-1.5 text-xs text-blue-900 dark:text-blue-200 shadow-xs",
                            div { class: "flex items-center gap-2 font-bold text-blue-700 dark:text-blue-400",
                                components::LucideIcon { name: "shield-check", class: "h-4 w-4 text-blue-600 dark:text-blue-400" }
                                "Enterprise Security & HIPAA Network Segmentation Rule"
                            }
                            p { class: "m-0 leading-relaxed text-[11px]",
                                "To comply with HIPAA §164.312(b) audit controls and hospital network segmentation rules, raw MLLP TCP socket listeners (port 2575) must run on dedicated Enterprise Server Gateways inside secure server VLANs. Desktop clients should use the interactive ER7 Sandbox below for manual testing and schema verification."
                            }
                        }

                        // Listeners List & Management
                        div { class: "space-y-3",
                            h3 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground m-0", "Configured MLLP Socket Listeners" }
                            if mllp_listeners.read().is_empty() {
                                div { class: "rounded-xl border border-dashed border-border p-6 text-center text-xs text-muted-foreground space-y-2",
                                    p { class: "m-0 font-medium", "No MLLP TCP Listeners configured for this workspace." }
                                    p { class: "m-0 text-[11px]", "Click 'New MLLP Listener' to start listening on standard port 2575 for hospital ADT, ORU, or ORM feeds." }
                                }
                            } else {
                                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                    for listener in mllp_listeners.read().iter() {
                                        {
                                            let l_id = listener.id.clone();
                                            let is_running = listener.status == "running";
                                            let dot_cls = if is_running { "h-2.5 w-2.5 rounded-full bg-emerald-500 animate-pulse" } else { "h-2.5 w-2.5 rounded-full bg-slate-400" };
                                            let status_cls = if is_running { "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase border bg-emerald-500/10 text-emerald-600 border-emerald-500/20" } else { "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase border bg-muted text-muted-foreground border-border" };
                                            rsx! {
                                                div { key: "{listener.id}", class: "rounded-xl border border-border bg-background p-4 space-y-3 shadow-xs",
                                                    div { class: "flex items-center justify-between",
                                                        div { class: "flex items-center gap-2",
                                                            div { class: "{dot_cls}" }
                                                            span { class: "font-bold text-xs text-foreground", "{listener.name}" }
                                                        }
                                                        span { class: "{status_cls}",
                                                            "{listener.status}"
                                                        }
                                                    }
                                                    div { class: "flex items-center gap-4 text-[11px] text-muted-foreground font-mono",
                                                        span { "Bind: {listener.bind_address}:{listener.port}" }
                                                        span { "TLS: {listener.tls_enabled}" }
                                                    }
                                                    div { class: "flex items-center justify-end gap-2 pt-2 border-t border-border/40",
                                                        if is_running {
                                                            button {
                                                                class: "px-3 py-1 text-xs font-semibold rounded-lg bg-amber-500/10 text-amber-600 hover:bg-amber-500/20 border border-amber-500/20 transition-all cursor-pointer",
                                                                onclick: {
                                                                    let u_id = active_user.id.clone();
                                                                    let w_id = workspace.id.clone();
                                                                    let lid = l_id.clone();
                                                                    let r_runner = runner.clone();
                                                                    move |_| {
                                                                        let u_id = u_id.clone();
                                                                        let w_id = w_id.clone();
                                                                        let lid = lid.clone();
                                                                        r_runner.clone().run(async move {
                                                                            stop_mllp_listener(u_id, w_id, lid).await?;
                                                                            db_trigger.with_mut(|v| *v += 1);
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "Stop Listener"
                                                            }
                                                        } else {
                                                            button {
                                                                class: "px-3 py-1 text-xs font-semibold rounded-lg bg-emerald-500/10 text-emerald-600 hover:bg-emerald-500/20 border border-emerald-500/20 transition-all cursor-pointer",
                                                                onclick: {
                                                                    let u_id = active_user.id.clone();
                                                                    let w_id = workspace.id.clone();
                                                                    let lid = l_id.clone();
                                                                    let r_runner = runner.clone();
                                                                    move |_| {
                                                                        let u_id = u_id.clone();
                                                                        let w_id = w_id.clone();
                                                                        let lid = lid.clone();
                                                                        r_runner.clone().run(async move {
                                                                            start_mllp_listener(u_id, w_id, lid).await?;
                                                                            db_trigger.with_mut(|v| *v += 1);
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "Start Listener"
                                                            }
                                                        }
                                                        button {
                                                            class: "px-3 py-1 text-xs font-semibold rounded-lg bg-destructive/10 text-destructive hover:bg-destructive/20 transition-all cursor-pointer",
                                                            onclick: {
                                                                let u_id = active_user.id.clone();
                                                                let w_id = workspace.id.clone();
                                                                let lid = l_id.clone();
                                                                let r_runner = runner.clone();
                                                                move |_| {
                                                                    let u_id = u_id.clone();
                                                                    let w_id = w_id.clone();
                                                                    let lid = lid.clone();
                                                                    r_runner.clone().run(async move {
                                                                        delete_mllp_listener(u_id, w_id, lid).await?;
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

                        // Test Ingestion Simulator
                        div { class: "rounded-xl border border-border bg-background p-4 space-y-3",
                            div { class: "flex items-center justify-between",
                                div {
                                    h3 { class: "text-xs font-bold text-foreground m-0 flex items-center gap-2",
                                        components::LucideIcon { name: "terminal", class: "h-4 w-4 text-primary" }
                                        "HL7 v2 Payload Simulator / Manual Ingestion"
                                    }
                                    p { class: "text-[11px] text-muted-foreground m-0 mt-0.5", "Test ER7 pipe-delimited segment parsing, patient MRN extraction, and MLLP ACK response generation." }
                                }
                                div { class: "flex items-center gap-2",
                                    button {
                                        class: "px-2.5 py-1 text-[11px] font-semibold rounded-lg border border-border bg-muted/40 hover:bg-muted text-foreground cursor-pointer",
                                        onclick: move |_| {
                                            test_er7_input.set("MSH|^~\\&|EPIC_EHR|GENERAL_HOSP|YNTRA_CORE|YNTRA_FAC|20260806140000||ADT^A01|CTRL1001|P|2.3\rPID|1||MRN778899^^^HOSP||SMITH^JANE^ELIZABETH||19920515|F|||456 PARK AVE^^SAN FRANCISCO^CA^94102||555-0199||\rPV1|1|I|ICU^BED-04^ROOM-101||||1234^STEVENS^MARK|||||||||||V998877|||||||||||||||||||||||||20260806140000|".to_string());
                                        },
                                        "ADT^A01 (Admission)"
                                    }
                                    button {
                                        class: "px-2.5 py-1 text-[11px] font-semibold rounded-lg border border-border bg-muted/40 hover:bg-muted text-foreground cursor-pointer",
                                        onclick: move |_| {
                                            test_er7_input.set("MSH|^~\\&|LAB_SYS|CENTRAL_LAB|YNTRA|YNTRA_FAC|20260806150000||ORU^R01|CTRL2002|P|2.3\rPID|1||MRN112233||BROWN^ROBERT||19751020|M\rOBR|1|ORD5544|LAB8877|CBC^COMPLETE BLOOD COUNT\rOBX|1|NM|WBC^WHITE BLOOD CELL COUNT||7.5|10^3/uL|4.5-11.0|N|||F\rOBX|2|NM|RBC^RED BLOOD CELL COUNT||4.8|10^6/uL|4.2-5.4|N|||F".to_string());
                                        },
                                        "ORU^R01 (Lab CBC)"
                                    }
                                    button {
                                        class: "px-2.5 py-1 text-[11px] font-semibold rounded-lg border border-border bg-muted/40 hover:bg-muted text-foreground cursor-pointer",
                                        onclick: move |_| {
                                            test_er7_input.set("MSH|^~\\&|ORDER_SYS|MAIN_HOSP|YNTRA|YNTRA_FAC|20260806160000||ORM^O01|CTRL3003|P|2.3\rPID|1||MRN445566||WILSON^SARAH\rORC|NW|ORD9988|||IP\rOBR|1|ORD9988||MRI_BRAIN^MRI BRAIN WITH CONTRAST".to_string());
                                        },
                                        "ORM^O01 (Order)"
                                    }
                                }
                            }
                            textarea {
                                class: "w-full h-24 rounded-xl border border-border bg-card p-2.5 text-xs font-mono text-foreground outline-none focus:ring-2 focus:ring-primary/20 resize-none",
                                value: "{test_er7_input}",
                                oninput: move |e| test_er7_input.set(e.value()),
                            }
                            div { class: "flex items-center justify-end",
                                button {
                                    class: "flex items-center gap-2 px-4 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer shadow-sm",
                                    onclick: {
                                        let w_id = workspace.id.clone();
                                        let r_runner = runner.clone();
                                        move |_| {
                                            let w_id = w_id.clone();
                                            let er7 = test_er7_input.read().clone();
                                            r_runner.clone().run(async move {
                                                process_raw_hl7_v2_message(w_id, None, er7).await?;
                                                db_trigger.with_mut(|v| *v += 1);
                                                Ok(())
                                            });
                                        }
                                    },
                                    components::LucideIcon { name: "send", class: "h-3.5 w-3.5" }
                                    "Ingest HL7 v2 ER7 Payload"
                                }
                            }
                        }

                        // Live Messages Table
                        div { class: "space-y-3",
                            h3 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground m-0", "Received HL7 v2 Ingestion Feed Log" }
                            if hl7_messages.read().is_empty() {
                                div { class: "rounded-xl border border-dashed border-border p-6 text-center text-xs text-muted-foreground",
                                    "No HL7 v2 messages received yet. Use the simulator above or connect an MLLP TCP stream to ingest feeds."
                                }
                            } else {
                                div { class: "overflow-x-auto rounded-xl border border-border",
                                    table { class: "w-full text-left text-xs border-collapse",
                                        thead { class: "bg-muted/50 text-muted-foreground font-semibold border-b border-border",
                                            tr {
                                                th { class: "p-3", "Type / Event" }
                                                th { class: "p-3", "Control ID" }
                                                th { class: "p-3", "Sending App/Facility" }
                                                th { class: "p-3", "Patient Name / MRN" }
                                                th { class: "p-3", "ACK Status" }
                                                th { class: "p-3 text-right", "Actions" }
                                            }
                                        }
                                        tbody { class: "divide-y divide-border/40",
                                            for msg in hl7_messages.read().iter() {
                                                {
                                                    let msg_clone = msg.clone();
                                                    let msg_event = format!("{}^{}", msg.message_type, msg.trigger_event);
                                                    let send_app = msg.sending_app.as_deref().unwrap_or("N/A");
                                                    let send_fac = msg.sending_facility.as_deref().unwrap_or("N/A");
                                                    let pat_name = msg.patient_name.as_deref().unwrap_or("Anonymous");
                                                    let pat_mrn = msg.patient_mrn.as_deref().unwrap_or("No MRN");
                                                    let ack_badge_cls = if msg.ack_status == "AA" { "px-2 py-0.5 rounded-full text-[10px] font-bold border bg-emerald-500/10 text-emerald-600 border-emerald-500/20" } else { "px-2 py-0.5 rounded-full text-[10px] font-bold border bg-destructive/10 text-destructive border-destructive/20" };
                                                    rsx! {
                                                        tr { key: "{msg.id}", class: "hover:bg-muted/30 transition-colors",
                                                            td { class: "p-3 font-bold text-foreground flex items-center gap-2",
                                                                span { class: "px-2 py-0.5 rounded-md text-[10px] font-mono bg-primary/10 text-primary border border-primary/20",
                                                                    "{msg_event}"
                                                                }
                                                            }
                                                            td { class: "p-3 font-mono text-muted-foreground text-[11px]", "{msg.message_control_id}" }
                                                            td { class: "p-3 text-muted-foreground text-[11px]",
                                                                "{send_app} / {send_fac}"
                                                            }
                                                            td { class: "p-3 font-medium text-foreground",
                                                                "{pat_name} ({pat_mrn})"
                                                            }
                                                            td { class: "p-3",
                                                                span { class: "{ack_badge_cls}",
                                                                    "ACK: {msg.ack_status}"
                                                                }
                                                            }
                                                            td { class: "p-3 text-right",
                                                                button {
                                                                    class: "px-3 py-1 text-xs font-semibold rounded-lg bg-secondary text-secondary-foreground hover:bg-secondary/80 border border-border transition-all cursor-pointer",
                                                                    onclick: move |_| selected_hl7_msg.set(Some(msg_clone.clone())),
                                                                    "Inspect Payload"
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

            // TAB 5: FHIR R4 Engine & Interoperability
            if active_tab.read().as_str() == "fhir" {
                div { class: "space-y-6",
                    // Header Banner
                    div { class: "rounded-2xl border border-primary/20 bg-gradient-to-br from-primary/5 via-background to-card p-6 shadow-sm flex flex-col md:flex-row md:items-center justify-between gap-4",
                        div { class: "space-y-1",
                            div { class: "flex items-center gap-2",
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-primary/10 text-primary border border-primary/20",
                                    "HL7 FHIR R4 Compliant Engine"
                                }
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                    "LOINC / SNOMED / ICD-10"
                                }
                            }
                            h3 { class: "text-lg font-bold text-foreground m-0", "Bidirectional FHIR R4 Resource Converter" }
                            p { class: "text-xs text-muted-foreground m-0 max-w-xl",
                                "Seamlessly transform native Yntra SQLite rows and Loro CRDT blobs into compliant HL7 FHIR R4 resources (/Patient, /Encounter, /Observation, /Condition, /Bundle) for Epic, Cerner, and MEDITECH integration."
                            }
                        }
                        div { class: "flex items-center gap-2 shrink-0",
                            button {
                                class: "flex items-center gap-2 px-4 py-2 text-xs font-semibold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer shadow-md shadow-primary/20",
                                onclick: {
                                    let u_id = active_user.id.clone();
                                    let w_id = workspace.id.clone();
                                    let r_runner = runner.clone();
                                    move |_| {
                                        let u_id = u_id.clone();
                                        let w_id = w_id.clone();
                                        r_runner.clone().run(async move {
                                            let json = export_workspace_fhir_bundle(u_id, w_id, Some("collection".to_string())).await?;
                                            exported_fhir_json.set(json);
                                            Ok(())
                                        });
                                    }
                                },
                                components::LucideIcon { name: "database", class: "h-4 w-4" }
                                "Export Workspace FHIR Bundle"
                            }
                        }
                    }

                    // Interactive Exporter & Importer Grid
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                        // Card 1: Outbound DB -> FHIR Exporter
                        div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4 flex flex-col",
                            div { class: "flex items-center justify-between border-b border-border/60 pb-3",
                                h3 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "arrow-up-right", class: "h-4 w-4 text-primary" }
                                    "Outbound DB Entity Exporter"
                                }
                                span { class: "text-[11px] font-mono text-muted-foreground", "Yntra DB -> FHIR R4 JSON" }
                            }
                            div { class: "space-y-3 flex-1",
                                div { class: "space-y-1.5",
                                    label { class: "text-xs font-semibold text-foreground", "Select Target FHIR Resource Type" }
                                    div { class: "grid grid-cols-4 gap-2",
                                        for rtype in ["Patient", "Encounter", "Observation", "Condition"].iter() {
                                            {
                                                let rtype_str = rtype.to_string();
                                                let is_selected = *selected_fhir_type.read() == rtype_str;
                                                let btn_cls = if is_selected {
                                                    "px-3 py-1.5 text-xs font-bold rounded-lg border bg-primary text-primary-foreground border-primary"
                                                } else {
                                                    "px-3 py-1.5 text-xs font-semibold rounded-lg border bg-background text-foreground border-border hover:bg-muted"
                                                };
                                                rsx! {
                                                    button {
                                                        key: "{rtype_str}",
                                                        class: "{btn_cls}",
                                                        onclick: move |_| selected_fhir_type.set(rtype_str.clone()),
                                                        "/{rtype}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "pt-2",
                                    button {
                                        class: "w-full py-2 px-4 text-xs font-bold rounded-xl bg-secondary text-secondary-foreground hover:bg-secondary/80 border border-border transition-all cursor-pointer flex items-center justify-center gap-2",
                                        onclick: {
                                            let u_id = active_user.id.clone();
                                            let w_id = workspace.id.clone();
                                            let r_runner = runner.clone();
                                            move |_| {
                                                let u_id = u_id.clone();
                                                let w_id = w_id.clone();
                                                let rtype = selected_fhir_type.read().clone();
                                                r_runner.clone().run(async move {
                                                    let json = match rtype.as_str() {
                                                        "Patient" => export_patient_to_fhir_r4(u_id, w_id, "cli-sample-1".to_string()).await?.json_payload,
                                                        "Encounter" => export_encounter_to_fhir_r4(u_id, w_id, "ticket-sample-1".to_string()).await?.json_payload,
                                                        "Observation" => export_observation_to_fhir_r4(u_id, w_id, "note-sample-1".to_string()).await?.json_payload,
                                                        _ => export_condition_to_fhir_r4(u_id, w_id, "cond-sample-1".to_string()).await?.json_payload,
                                                    };
                                                    exported_fhir_json.set(json);
                                                    db_trigger.with_mut(|v| *v += 1);
                                                    Ok(())
                                                });
                                            }
                                        },
                                        components::LucideIcon { name: "file-text", class: "h-4 w-4" }
                                        "Generate FHIR R4 /{selected_fhir_type} Resource"
                                    }
                                }

                                if !exported_fhir_json.read().is_empty() {
                                    div { class: "space-y-1.5 pt-2",
                                        label { class: "text-[11px] font-bold uppercase tracking-wider text-muted-foreground", "Generated FHIR R4 JSON Output" }
                                        pre { class: "p-3 rounded-xl bg-muted/60 border border-border font-mono text-[11px] text-foreground overflow-x-auto max-h-60 overflow-y-auto whitespace-pre-wrap m-0",
                                            "{exported_fhir_json}"
                                        }
                                    }
                                }
                            }
                        }

                        // Card 2: Inbound FHIR Importer & Validator
                        div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4 flex flex-col",
                            div { class: "flex items-center justify-between border-b border-border/60 pb-3",
                                h3 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "arrow-down-left", class: "h-4 w-4 text-emerald-500" }
                                    "Inbound FHIR R4 Payload Importer"
                                }
                                span { class: "text-[11px] font-mono text-muted-foreground", "External FHIR -> Yntra DB" }
                            }
                            div { class: "space-y-3 flex-1",
                                div { class: "space-y-1.5",
                                    label { class: "text-xs font-semibold text-foreground", "Raw FHIR R4 JSON Payload" }
                                    textarea {
                                        class: "w-full h-36 p-3 rounded-xl bg-background border border-border font-mono text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none",
                                        value: "{import_fhir_input}",
                                        oninput: move |e| import_fhir_input.set(e.value())
                                    }
                                }

                                if let Some((success, msg)) = import_fhir_status.read().as_ref() {
                                    {
                                        let status_alert_cls = if *success { "p-3 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 text-xs font-medium" } else { "p-3 rounded-xl border border-destructive/30 bg-destructive/10 text-destructive text-xs font-medium" };
                                        rsx! {
                                            div { class: "{status_alert_cls}", "{msg}" }
                                        }
                                    }
                                }

                                div { class: "pt-2",
                                    button {
                                        class: "w-full py-2.5 px-4 text-xs font-bold rounded-xl bg-emerald-600 text-white hover:bg-emerald-700 transition-all cursor-pointer flex items-center justify-center gap-2 shadow-sm",
                                        onclick: {
                                            let u_id = active_user.id.clone();
                                            let w_id = workspace.id.clone();
                                            let r_runner = runner.clone();
                                            move |_| {
                                                let u_id = u_id.clone();
                                                let w_id = w_id.clone();
                                                let fhir_raw = import_fhir_input.read().clone();
                                                r_runner.clone().run(async move {
                                                    match import_fhir_r4_resource(u_id, w_id, fhir_raw).await {
                                                        Ok(res) => {
                                                            import_fhir_status.set(Some((true, res.message)));
                                                            db_trigger.with_mut(|v| *v += 1);
                                                        }
                                                        Err(e) => {
                                                            import_fhir_status.set(Some((false, format!("Validation Failed: {}", e))));
                                                        }
                                                    }
                                                    Ok(())
                                                });
                                            }
                                        },
                                        components::LucideIcon { name: "check-circle", class: "h-4 w-4" }
                                        "Validate & Import FHIR Resource into DB"
                                    }
                                }
                            }
                        }
                    }

                    // Stored FHIR Resource Mappings Table
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                        div { class: "flex items-center justify-between",
                            h3 { class: "text-md font-bold text-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "link-2", class: "h-4 w-4 text-primary" }
                                "Stored FHIR R4 Resource Mappings"
                            }
                            span { class: "text-xs text-muted-foreground font-mono", "{fhir_mappings.read().len()} Active Mappings" }
                        }

                        if fhir_mappings.read().is_empty() {
                            div { class: "p-8 text-center border border-dashed border-border rounded-xl space-y-2",
                                components::LucideIcon { name: "database", class: "h-8 w-8 mx-auto text-muted-foreground/50" }
                                p { class: "text-xs text-muted-foreground italic m-0", "No FHIR R4 resource mappings recorded yet. Generate or import resources above to track mappings." }
                            }
                        } else {
                            div { class: "overflow-x-auto",
                                table { class: "w-full text-left text-xs border-collapse",
                                    thead { class: "border-b border-border bg-muted/30 text-muted-foreground uppercase text-[10px] tracking-wider",
                                        tr {
                                            th { class: "p-3 font-semibold", "Resource Type" }
                                            th { class: "p-3 font-semibold", "FHIR Resource ID" }
                                            th { class: "p-3 font-semibold", "Internal Entity Type" }
                                            th { class: "p-3 font-semibold", "Internal Entity ID" }
                                            th { class: "p-3 font-semibold", "Last Synced At" }
                                        }
                                    }
                                    tbody { class: "divide-y divide-border/40",
                                        for mapping in fhir_mappings.read().iter() {
                                            {
                                                let mapping_id = mapping.id.clone();
                                                rsx! {
                                                    tr { key: "{mapping_id}", class: "hover:bg-muted/30 transition-colors",
                                                        td { class: "p-3 font-bold text-foreground flex items-center gap-2",
                                                            span { class: "px-2 py-0.5 rounded-md text-[10px] font-mono bg-primary/10 text-primary border border-primary/20",
                                                                "/{mapping.resource_type}"
                                                            }
                                                        }
                                                        td { class: "p-3 font-mono text-foreground text-[11px]", "{mapping.fhir_id}" }
                                                        td { class: "p-3 text-muted-foreground text-[11px]", "{mapping.internal_entity_type}" }
                                                        td { class: "p-3 font-mono text-muted-foreground text-[11px]", "{mapping.internal_entity_id}" }
                                                        td { class: "p-3 text-muted-foreground text-[11px]", "{mapping.last_synced_at}" }
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

            // TAB 6: NCPDP SCRIPT e-Prescribing & Surescripts Integration
            if active_tab.read().as_str() == "ncpdp" {
                div { class: "space-y-6",
                    // Header Banner
                    div { class: "rounded-2xl border border-primary/20 bg-gradient-to-br from-primary/5 via-background to-card p-6 shadow-sm flex flex-col md:flex-row md:items-center justify-between gap-4",
                        div { class: "space-y-1",
                            div { class: "flex items-center gap-2",
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-primary/10 text-primary border border-primary/20",
                                    "NCPDP SCRIPT v2017071 Standard"
                                }
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                    "Surescripts Network Integration"
                                }
                            }
                            h3 { class: "text-lg font-bold text-foreground m-0", "Electronic Prescribing (e-Prescribing) Engine" }
                            p { class: "text-xs text-muted-foreground m-0 max-w-xl",
                                "Format, validate, and transmit electronic prescriptions (NewRx, CancelRx, RxRenewal) directly to Surescripts and retail pharmacy networks with full NPI and DEA validation."
                            }
                        }
                        div { class: "flex items-center gap-2 shrink-0",
                            span { class: "flex items-center gap-2 px-3 py-1.5 rounded-xl bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-xs font-semibold",
                                components::LucideIcon { name: "wifi", class: "h-4 w-4" }
                                "Surescripts EDI Gateway: Connected"
                            }
                        }
                    }

                    // Interactive Composer & Inbound Importer Grid
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                        // Card 1: Interactive NewRx e-Prescription Composer
                        div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4 flex flex-col",
                            div { class: "flex items-center justify-between border-b border-border/60 pb-3",
                                h3 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "file-text", class: "h-4 w-4 text-primary" }
                                    "Outbound NewRx e-Prescription Composer"
                                }
                                span { class: "text-[11px] font-mono text-muted-foreground", "NCPDP v2017071 XML" }
                            }
                            div { class: "space-y-3 flex-1 text-xs",
                                div { class: "grid grid-cols-2 gap-3",
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Prescriber NPI (10-Digit)" }
                                        input {
                                            class: "w-full p-2 rounded-xl bg-background border border-border font-mono text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                            value: "{ncpdp_prescriber_npi}",
                                            oninput: move |e| ncpdp_prescriber_npi.set(e.value())
                                        }
                                    }
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Pharmacy NPI (10-Digit)" }
                                        input {
                                            class: "w-full p-2 rounded-xl bg-background border border-border font-mono text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                            value: "{ncpdp_pharmacy_npi}",
                                            oninput: move |e| ncpdp_pharmacy_npi.set(e.value())
                                        }
                                    }
                                }

                                div { class: "space-y-1",
                                    label { class: "font-semibold text-foreground", "Medication / Drug Name" }
                                    input {
                                        class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                        value: "{ncpdp_drug_name}",
                                        oninput: move |e| ncpdp_drug_name.set(e.value())
                                    }
                                }

                                div { class: "grid grid-cols-3 gap-3",
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Quantity" }
                                        input {
                                            type: "number",
                                            class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                            value: "{ncpdp_quantity}",
                                            oninput: move |e| {
                                                if let Ok(v) = e.value().parse::<f64>() {
                                                    ncpdp_quantity.set(v);
                                                }
                                            }
                                        }
                                    }
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Days Supply" }
                                        input {
                                            type: "number",
                                            class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                            value: "{ncpdp_days_supply}",
                                            oninput: move |e| {
                                                if let Ok(v) = e.value().parse::<u32>() {
                                                    ncpdp_days_supply.set(v);
                                                }
                                            }
                                        }
                                    }
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Refills" }
                                        input {
                                            type: "number",
                                            class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                            value: "{ncpdp_refills}",
                                            oninput: move |e| {
                                                if let Ok(v) = e.value().parse::<u32>() {
                                                    ncpdp_refills.set(v);
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "space-y-1",
                                    label { class: "font-semibold text-foreground", "Sig (Directions for Use)" }
                                    input {
                                        class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                        value: "{ncpdp_sig}",
                                        oninput: move |e| ncpdp_sig.set(e.value())
                                    }
                                }

                                div { class: "pt-2",
                                    button {
                                        class: "w-full py-2.5 px-4 text-xs font-bold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer flex items-center justify-center gap-2 shadow-sm",
                                        onclick: {
                                            let u_id = active_user.id.clone();
                                            let w_id = workspace.id.clone();
                                            let r_runner = runner.clone();
                                            move |_| {
                                                let u_id = u_id.clone();
                                                let w_id = w_id.clone();
                                                let drug = ncpdp_drug_name.read().clone();
                                                let p_npi = ncpdp_prescriber_npi.read().clone();
                                                let ph_npi = ncpdp_pharmacy_npi.read().clone();
                                                let qty = *ncpdp_quantity.read();
                                                let days = *ncpdp_days_supply.read();
                                                let refs = *ncpdp_refills.read();
                                                let sig_text = ncpdp_sig.read().clone();

                                                r_runner.clone().run(async move {
                                                    match create_ncpdp_new_rx_prescription(
                                                        u_id,
                                                        w_id,
                                                        "cli-patient-1".to_string(),
                                                        p_npi,
                                                        ph_npi,
                                                        Some("Walgreens Pharmacy".to_string()),
                                                        drug,
                                                        Some("RxNorm-308182".to_string()),
                                                        Some("NDC-00093-3109".to_string()),
                                                        qty,
                                                        days,
                                                        refs,
                                                        sig_text,
                                                        None,
                                                        None,
                                                        None,
                                                    ).await {
                                                        Ok(res) => {
                                                            generated_ncpdp_xml.set(res.xml_payload);
                                                            ncpdp_status_alert.set(Some((true, res.message)));
                                                            db_trigger.with_mut(|v| *v += 1);
                                                        }
                                                        Err(e) => {
                                                            ncpdp_status_alert.set(Some((false, format!("Validation Error: {}", e))));
                                                        }
                                                    }
                                                    Ok(())
                                                });
                                            }
                                        },
                                        components::LucideIcon { name: "plus-circle", class: "h-4 w-4" }
                                        "Generate & Create NewRx XML Draft"
                                    }
                                }

                                if !generated_ncpdp_xml.read().is_empty() {
                                    div { class: "space-y-1.5 pt-2",
                                        label { class: "text-[11px] font-bold uppercase tracking-wider text-muted-foreground", "Generated NCPDP SCRIPT v2017071 XML Output" }
                                        pre { class: "p-3 rounded-xl bg-muted/60 border border-border font-mono text-[11px] text-foreground overflow-x-auto max-h-56 overflow-y-auto whitespace-pre-wrap m-0",
                                            "{generated_ncpdp_xml}"
                                        }
                                    }
                                }
                            }
                        }

                        // Card 2: Inbound Pharmacy EDI XML Inspector
                        div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4 flex flex-col",
                            div { class: "flex items-center justify-between border-b border-border/60 pb-3",
                                h3 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "file-code", class: "h-4 w-4 text-emerald-500" }
                                    "Inbound Pharmacy EDI XML Inspector"
                                }
                                span { class: "text-[11px] font-mono text-muted-foreground", "Pharmacy -> Prescriber EDI" }
                            }
                            div { class: "space-y-3 flex-1 text-xs",
                                div { class: "space-y-1.5",
                                    label { class: "font-semibold text-foreground", "Raw Pharmacy NCPDP EDI XML Payload" }
                                    textarea {
                                        class: "w-full h-44 p-3 rounded-xl bg-background border border-border font-mono text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none",
                                        value: "{inbound_ncpdp_input}",
                                        oninput: move |e| inbound_ncpdp_input.set(e.value())
                                    }
                                }

                                if let Some((success, msg)) = ncpdp_status_alert.read().as_ref() {
                                    {
                                        let alert_cls = if *success { "p-3 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 text-xs font-medium" } else { "p-3 rounded-xl border border-destructive/30 bg-destructive/10 text-destructive text-xs font-medium" };
                                        rsx! {
                                            div { class: "{alert_cls}", "{msg}" }
                                        }
                                    }
                                }

                                div { class: "pt-2",
                                    button {
                                        class: "w-full py-2.5 px-4 text-xs font-bold rounded-xl bg-emerald-600 text-white hover:bg-emerald-700 transition-all cursor-pointer flex items-center justify-center gap-2 shadow-sm",
                                        onclick: {
                                            let u_id = active_user.id.clone();
                                            let w_id = workspace.id.clone();
                                            let r_runner = runner.clone();
                                            move |_| {
                                                let u_id = u_id.clone();
                                                let w_id = w_id.clone();
                                                let xml_raw = inbound_ncpdp_input.read().clone();
                                                r_runner.clone().run(async move {
                                                    match parse_and_import_inbound_ncpdp_xml(u_id, w_id, xml_raw).await {
                                                        Ok(res) => {
                                                            ncpdp_status_alert.set(Some((true, res.message)));
                                                            db_trigger.with_mut(|v| *v += 1);
                                                        }
                                                        Err(e) => {
                                                            ncpdp_status_alert.set(Some((false, format!("EDI Parsing Error: {}", e))));
                                                        }
                                                    }
                                                    Ok(())
                                                });
                                            }
                                        },
                                        components::LucideIcon { name: "check-circle", class: "h-4 w-4" }
                                        "Parse & Record Pharmacy EDI Payload"
                                    }
                                }
                            }
                        }
                    }

                    // Active Electronic Prescriptions Log Table
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                        div { class: "flex items-center justify-between",
                            h3 { class: "text-md font-bold text-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "activity", class: "h-4 w-4 text-primary" }
                                "Electronic Prescriptions Audit Log"
                            }
                            span { class: "text-xs text-muted-foreground font-mono", "{ncpdp_prescriptions.read().len()} Total Prescriptions" }
                        }

                        if ncpdp_prescriptions.read().is_empty() {
                            div { class: "p-8 text-center border border-dashed border-border rounded-xl space-y-2",
                                components::LucideIcon { name: "file-text", class: "h-8 w-8 mx-auto text-muted-foreground/50" }
                                p { class: "text-xs text-muted-foreground italic m-0", "No electronic prescriptions recorded yet. Compose a NewRx prescription above to begin." }
                            }
                        } else {
                            div { class: "overflow-x-auto",
                                table { class: "w-full text-left text-xs border-collapse",
                                    thead { class: "border-b border-border bg-muted/30 text-muted-foreground uppercase text-[10px] tracking-wider",
                                        tr {
                                            th { class: "p-3 font-semibold", "Prescription ID" }
                                            th { class: "p-3 font-semibold", "Medication / Drug" }
                                            th { class: "p-3 font-semibold", "Prescriber / Pharmacy NPI" }
                                            th { class: "p-3 font-semibold", "Type" }
                                            th { class: "p-3 font-semibold", "Status" }
                                            th { class: "p-3 font-semibold", "Surescripts Tx ID" }
                                            th { class: "p-3 font-semibold text-right", "Actions" }
                                        }
                                    }
                                    tbody { class: "divide-y divide-border/40",
                                        for rx in ncpdp_prescriptions.read().iter() {
                                            {
                                                let rx_id = rx.id.clone();
                                                let rx_clone = rx.clone();
                                                let is_draft = rx.status == "draft";
                                                let is_transmitted = rx.status == "transmitted";
                                                let status_badge = match rx.status.as_str() {
                                                    "transmitted" => "px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                                    "cancelled" => "px-2 py-0.5 rounded-full text-[10px] font-bold bg-destructive/10 text-destructive border border-destructive/20",
                                                    "received" => "px-2 py-0.5 rounded-full text-[10px] font-bold bg-blue-500/10 text-blue-600 border border-blue-500/20",
                                                    _ => "px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20",
                                                };
                                                let surescripts_id = rx.surescripts_tx_id.as_deref().unwrap_or("-");

                                                rsx! {
                                                    tr { key: "{rx_id}", class: "hover:bg-muted/30 transition-colors",
                                                        td { class: "p-3 font-mono font-bold text-foreground text-[11px]", "{rx.id}" }
                                                        td { class: "p-3 font-medium text-foreground", "{rx.drug_name}" }
                                                        td { class: "p-3 text-muted-foreground text-[11px] font-mono",
                                                            "{rx.prescriber_npi} -> {rx.pharmacy_npi}"
                                                        }
                                                        td { class: "p-3 font-semibold text-foreground", "{rx.transaction_type}" }
                                                        td { class: "p-3",
                                                            span { class: "{status_badge}", "{rx.status}" }
                                                        }
                                                        td { class: "p-3 font-mono text-muted-foreground text-[11px]", "{surescripts_id}" }
                                                        td { class: "p-3 text-right space-x-1.5",
                                                            if is_draft {
                                                                button {
                                                                    class: "px-3 py-1 text-xs font-bold rounded-lg bg-emerald-600 text-white hover:bg-emerald-700 transition-all cursor-pointer border-0 shadow-xs",
                                                                    onclick: {
                                                                        let u_id = active_user.id.clone();
                                                                        let w_id = workspace.id.clone();
                                                                        let r_id = rx_clone.id.clone();
                                                                        let r_runner = runner.clone();
                                                                        move |_| {
                                                                            let u_id = u_id.clone();
                                                                            let w_id = w_id.clone();
                                                                            let r_id = r_id.clone();
                                                                            r_runner.clone().run(async move {
                                                                                let res = transmit_ncpdp_script_to_surescripts(u_id, w_id, r_id, None).await?;
                                                                                ncpdp_status_alert.set(Some((true, res.message)));
                                                                                db_trigger.with_mut(|v| *v += 1);
                                                                                Ok(())
                                                                            });
                                                                        }
                                                                    },
                                                                    "Transmit EDI"
                                                                }
                                                            }
                                                            if is_transmitted {
                                                                button {
                                                                    class: "px-3 py-1 text-xs font-semibold rounded-lg bg-destructive/10 text-destructive hover:bg-destructive/20 transition-all cursor-pointer border border-destructive/20",
                                                                    onclick: {
                                                                        let u_id = active_user.id.clone();
                                                                        let w_id = workspace.id.clone();
                                                                        let r_id = rx_clone.id.clone();
                                                                        let r_runner = runner.clone();
                                                                        move |_| {
                                                                            let u_id = u_id.clone();
                                                                            let w_id = w_id.clone();
                                                                            let r_id = r_id.clone();
                                                                            r_runner.clone().run(async move {
                                                                                let res = cancel_ncpdp_prescription(u_id, w_id, r_id, "Prescriber requested cancellation".to_string()).await?;
                                                                                ncpdp_status_alert.set(Some((true, res.message)));
                                                                                db_trigger.with_mut(|v| *v += 1);
                                                                                Ok(())
                                                                            });
                                                                        }
                                                                    },
                                                                    "CancelRx"
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

            // TAB 7: FDA 21 CFR Part 11 Electronic Signatures
            if active_tab.read().as_str() == "fda_part11" {
                div { class: "space-y-6",
                    // Header Banner
                    div { class: "rounded-2xl border border-primary/20 bg-gradient-to-br from-primary/5 via-background to-card p-6 shadow-sm flex flex-col md:flex-row md:items-center justify-between gap-4",
                        div { class: "space-y-1",
                            div { class: "flex items-center gap-2",
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-primary/10 text-primary border border-primary/20",
                                    "FDA 21 CFR Part 11 Subpart C"
                                }
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-emerald-500/10 text-emerald-600 border border-emerald-500/20",
                                    "Dual-Credential Ed25519 Binding"
                                }
                            }
                            h3 { class: "text-lg font-bold text-foreground m-0", "FDA 21 CFR Part 11 Electronic Signature Engine" }
                            p { class: "text-xs text-muted-foreground m-0 max-w-xl",
                                "Cryptographic non-repudiation signature binding for clinical chart edits, lab approvals, and controlled prescriptions with dual-person cosigning supervisor authorization."
                            }
                        }
                        div { class: "flex items-center gap-2 shrink-0",
                            span { class: "flex items-center gap-2 px-3 py-1.5 rounded-xl bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-xs font-semibold",
                                components::LucideIcon { name: "shield-check", class: "h-4 w-4" }
                                "Audit Trail Non-Repudiation: Active"
                            }
                        }
                    }

                    // Interactive Dual-Signer Execution Form
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                        div { class: "flex items-center justify-between border-b border-border/60 pb-3",
                            h3 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "pen-tool", class: "h-4 w-4 text-primary" }
                                "FDA Electronic Signature Execution & Intent Binding"
                            }
                            span { class: "text-[11px] font-mono text-muted-foreground", "Dual-Person Cosignature Protocol" }
                        }

                        div { class: "space-y-4 text-xs",
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div { class: "space-y-1",
                                    label { class: "font-semibold text-foreground", "Target Record Category" }
                                    select {
                                        class: "w-full p-2.5 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                        value: "{fda_target_type}",
                                        onchange: move |e| fda_target_type.set(e.value()),
                                        option { value: "clinical_chart", "Clinical Chart Revision / Medical Note" }
                                        option { value: "lab_approval", "Laboratory Test Approval & Sign-off" }
                                        option { value: "ncpdp_prescription", "Controlled Drug Prescription (NewRx/CancelRx)" }
                                        option { value: "dosage_change", "Medication Dosage / Care Level Adjustment" }
                                    }
                                }
                                div { class: "space-y-1",
                                    label { class: "font-semibold text-foreground", "Target Record ID" }
                                    input {
                                        class: "w-full p-2.5 rounded-xl bg-background border border-border font-mono text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                        value: "{fda_target_id}",
                                        oninput: move |e| fda_target_id.set(e.value())
                                    }
                                }
                            }

                            // Primary Authorizer Section
                            div { class: "p-4 rounded-xl bg-muted/30 border border-border space-y-3",
                                h4 { class: "text-xs font-bold text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "user-check", class: "h-4 w-4 text-primary" }
                                    "Primary Authorizer (Clinician / Attending Physician)"
                                }
                                div { class: "grid grid-cols-1 md:grid-cols-2 gap-3",
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Signer Printed Name (21 CFR §11.50)" }
                                        input {
                                            class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                            value: "{fda_primary_name}",
                                            oninput: move |e| fda_primary_name.set(e.value())
                                        }
                                    }
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Manifested Intent (Authorship / Entry)" }
                                        input {
                                            class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                            value: "{fda_primary_intent}",
                                            oninput: move |e| fda_primary_intent.set(e.value())
                                        }
                                    }
                                }
                            }

                            // Secondary Supervisor Cosigner Section
                            div { class: "p-4 rounded-xl bg-muted/30 border border-border space-y-3",
                                div { class: "flex items-center justify-between",
                                    h4 { class: "text-xs font-bold text-foreground m-0 flex items-center gap-2",
                                        components::LucideIcon { name: "users", class: "h-4 w-4 text-emerald-500" }
                                        "Secondary Cosigner (Supervising Medical Officer / Pharmacist)"
                                    }
                                    label { class: "flex items-center gap-2 cursor-pointer font-semibold text-foreground text-xs",
                                        input {
                                            type: "checkbox",
                                            checked: "{fda_require_dual}",
                                            onchange: move |e| fda_require_dual.set(e.value().parse().unwrap_or(false))
                                        }
                                        "Require Dual-Person Cosignature"
                                    }
                                }

                                if *fda_require_dual.read() {
                                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-3 pt-1",
                                        div { class: "space-y-1",
                                            label { class: "font-semibold text-foreground", "Supervisor User ID" }
                                            input {
                                                class: "w-full p-2 rounded-xl bg-background border border-border font-mono text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                                value: "{fda_secondary_user_id}",
                                                oninput: move |e| fda_secondary_user_id.set(e.value())
                                            }
                                        }
                                        div { class: "space-y-1",
                                            label { class: "font-semibold text-foreground", "Supervisor Printed Name" }
                                            input {
                                                class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                                value: "{fda_secondary_name}",
                                                oninput: move |e| fda_secondary_name.set(e.value())
                                            }
                                        }
                                        div { class: "space-y-1",
                                            label { class: "font-semibold text-foreground", "Cosigning Manifested Intent" }
                                            input {
                                                class: "w-full p-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20",
                                                value: "{fda_secondary_intent}",
                                                oninput: move |e| fda_secondary_intent.set(e.value())
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some((success, msg)) = fda_status_alert.read().as_ref() {
                                {
                                    let alert_cls = if *success { "p-3 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 text-xs font-medium" } else { "p-3 rounded-xl border border-destructive/30 bg-destructive/10 text-destructive text-xs font-medium" };
                                    rsx! {
                                        div { class: "{alert_cls}", "{msg}" }
                                    }
                                }
                            }

                            div { class: "pt-2",
                                button {
                                    class: "w-full py-3 px-4 text-xs font-bold rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 transition-all cursor-pointer flex items-center justify-center gap-2 shadow-sm",
                                    onclick: {
                                        let u_id = active_user.id.clone();
                                        let w_id = workspace.id.clone();
                                        let r_runner = runner.clone();
                                        move |_| {
                                            let u_id = u_id.clone();
                                            let w_id = w_id.clone();
                                            let t_type = fda_target_type.read().clone();
                                            let t_id = fda_target_id.read().clone();
                                            let p_name = fda_primary_name.read().clone();
                                            let p_intent = fda_primary_intent.read().clone();
                                            let req_dual = *fda_require_dual.read();
                                            let s_uid = if req_dual { Some(fda_secondary_user_id.read().clone()) } else { None };
                                            let s_name = if req_dual { Some(fda_secondary_name.read().clone()) } else { None };
                                            let s_intent = if req_dual { Some(fda_secondary_intent.read().clone()) } else { None };

                                            r_runner.clone().run(async move {
                                                match execute_fda_part11_dual_signature(
                                                    u_id,
                                                    w_id,
                                                    t_type,
                                                    t_id,
                                                    p_name,
                                                    p_intent,
                                                    s_uid,
                                                    s_name,
                                                    s_intent,
                                                ).await {
                                                    Ok(res) => {
                                                        fda_status_alert.set(Some((true, res.message)));
                                                        db_trigger.with_mut(|v| *v += 1);
                                                    }
                                                    Err(e) => {
                                                        fda_status_alert.set(Some((false, format!("FDA Part 11 Execution Error: {}", e))));
                                                    }
                                                }
                                                Ok(())
                                            });
                                        }
                                    },
                                    components::LucideIcon { name: "lock", class: "h-4 w-4" }
                                    "Execute FDA 21 CFR Part 11 Cryptographic Signature"
                                }
                            }
                        }
                    }

                    // FDA Electronic Signatures Audit Log Table
                    div { class: "rounded-2xl border border-border bg-card p-6 shadow-sm space-y-4",
                        div { class: "flex items-center justify-between",
                            h3 { class: "text-md font-bold text-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "shield", class: "h-4 w-4 text-emerald-500" }
                                "FDA 21 CFR Part 11 Non-Repudiation Audit Logs"
                            }
                            span { class: "text-xs text-muted-foreground font-mono", "{fda_signatures.read().len()} Signed Records" }
                        }

                        if fda_signatures.read().is_empty() {
                            div { class: "p-8 text-center border border-dashed border-border rounded-xl space-y-2",
                                components::LucideIcon { name: "shield-alert", class: "h-8 w-8 mx-auto text-muted-foreground/50" }
                                p { class: "text-xs text-muted-foreground italic m-0", "No FDA electronic signatures executed yet. Use the form above to generate a dual-person signed audit record." }
                            }
                        } else {
                            div { class: "overflow-x-auto",
                                table { class: "w-full text-left text-xs border-collapse",
                                    thead { class: "border-b border-border bg-muted/30 text-muted-foreground uppercase text-[10px] tracking-wider",
                                        tr {
                                            th { class: "p-3 font-semibold", "Signature ID" }
                                            th { class: "p-3 font-semibold", "Target Category / ID" }
                                            th { class: "p-3 font-semibold", "Primary Authorizer" }
                                            th { class: "p-3 font-semibold", "Secondary Cosigner" }
                                            th { class: "p-3 font-semibold", "Dual Signed" }
                                            th { class: "p-3 font-semibold text-right", "Actions" }
                                        }
                                    }
                                    tbody { class: "divide-y divide-border/40",
                                        for sig in fda_signatures.read().iter() {
                                            {
                                                let sig_id = sig.id.clone();
                                                let sig_clone = sig.clone();
                                                let dual_badge = if sig.dual_sign_completed {
                                                    "px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-500/10 text-emerald-600 border border-emerald-500/20"
                                                } else {
                                                    "px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20"
                                                };
                                                let sec_name = sig.secondary_signer_name.as_deref().unwrap_or("N/A");
                                                let sec_intent = sig.secondary_intent.as_deref().unwrap_or("-");

                                                rsx! {
                                                    tr { key: "{sig_id}", class: "hover:bg-muted/30 transition-colors",
                                                        td { class: "p-3 font-mono font-bold text-foreground text-[11px]", "{sig.id}" }
                                                        td { class: "p-3 text-foreground font-medium",
                                                            div { class: "font-semibold text-foreground", "{sig.target_record_type}" }
                                                            div { class: "text-[11px] font-mono text-muted-foreground", "{sig.target_record_id}" }
                                                        }
                                                        td { class: "p-3 text-foreground",
                                                            div { class: "font-medium", "{sig.primary_signer_name}" }
                                                            div { class: "text-[11px] text-muted-foreground italic", "{sig.primary_intent}" }
                                                        }
                                                        td { class: "p-3 text-muted-foreground",
                                                            div { class: "font-medium text-foreground", "{sec_name}" }
                                                            div { class: "text-[11px] text-muted-foreground italic", "{sec_intent}" }
                                                        }
                                                        td { class: "p-3",
                                                            span { class: "{dual_badge}",
                                                                if sig.dual_sign_completed { "DUAL SIGNED" } else { "SINGLE SIGNED" }
                                                            }
                                                        }
                                                        td { class: "p-3 text-right",
                                                            button {
                                                                class: "px-3 py-1 text-xs font-semibold rounded-lg bg-primary/10 text-primary hover:bg-primary/20 transition-all cursor-pointer border border-primary/20",
                                                                onclick: {
                                                                    let u_id = active_user.id.clone();
                                                                    let w_id = workspace.id.clone();
                                                                    let s_id = sig_clone.id.clone();
                                                                    let r_runner = runner.clone();
                                                                    move |_| {
                                                                        let u_id = u_id.clone();
                                                                        let w_id = w_id.clone();
                                                                        let s_id = s_id.clone();
                                                                        r_runner.clone().run(async move {
                                                                            let res = verify_fda_part11_dual_signature(u_id, w_id, s_id).await?;
                                                                            fda_status_alert.set(Some((res.is_valid, res.message)));
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "Verify Ed25519"
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

            // MODAL 3: New MLLP Listener Modal
            if *show_mllp_modal.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4",
                    div { class: "w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-4",
                        h3 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "activity", class: "h-5 w-5 text-primary" }
                            "Create MLLP TCP Listener"
                        }
                        div { class: "space-y-3 text-xs",
                            div { class: "space-y-1",
                                label { class: "font-semibold text-foreground", "Listener Name" }
                                input {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-medium text-foreground outline-none",
                                    value: "{mllp_name}",
                                    oninput: move |e| mllp_name.set(e.value()),
                                    placeholder: "Hospital ADT MLLP Feed",
                                }
                            }
                            div { class: "space-y-1",
                                label { class: "font-semibold text-foreground", "TCP Socket Port" }
                                input {
                                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs font-mono font-medium text-foreground outline-none",
                                    value: "{mllp_port}",
                                    oninput: move |e| {
                                        if let Ok(p) = e.value().parse::<u16>() {
                                            mllp_port.set(p);
                                        }
                                    },
                                    placeholder: "2575",
                                }
                            }
                        }
                        div { class: "flex items-center justify-end gap-3 pt-4 border-t border-border",
                            button {
                                class: "px-4 py-2 text-xs font-semibold rounded-xl border border-border bg-transparent text-muted-foreground hover:bg-muted cursor-pointer",
                                onclick: move |_| show_mllp_modal.set(false),
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
                                        let name = mllp_name.read().clone();
                                        let port = *mllp_port.read();
                                        r_runner.clone().run(async move {
                                            let config = MllpListenerConfig {
                                                id: "".to_string(),
                                                workspace_id: w_id,
                                                name,
                                                port,
                                                bind_address: "0.0.0.0".to_string(),
                                                tls_enabled: false,
                                                status: "stopped".to_string(),
                                                last_active_at: 0,
                                                created_at: 0,
                                                updated_at: 0,
                                            };
                                            save_mllp_listener(u_id, config).await?;
                                            show_mllp_modal.set(false);
                                            db_trigger.with_mut(|v| *v += 1);
                                            Ok(())
                                        });
                                    }
                                },
                                "Create Listener"
                            }
                        }
                    }
                }
            }

            // MODAL 4: Payload & ACK Inspector Modal
            if let Some(msg) = selected_hl7_msg.read().as_ref() {
                {
                    let raw_er7 = msg.raw_payload.clone();
                    let parsed_json = msg.parsed_json.clone();
                    let ack_payload = msg.ack_payload.clone().unwrap_or_default();
                    rsx! {
                        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4",
                            div { class: "w-full max-w-2xl rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-4 max-h-[85vh] flex flex-col",
                                div { class: "flex items-center justify-between border-b border-border pb-3",
                                    h3 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                        components::LucideIcon { name: "file-code", class: "h-5 w-5 text-primary" }
                                        "HL7 v2 Message Payload Inspector"
                                    }
                                    button {
                                        class: "text-muted-foreground hover:text-foreground cursor-pointer border-0 bg-transparent text-lg font-bold",
                                        onclick: move |_| selected_hl7_msg.set(None),
                                        "×"
                                    }
                                }
                                div { class: "overflow-y-auto space-y-4 pr-1 text-xs flex-1",
                                    div { class: "space-y-1.5",
                                        label { class: "font-bold text-foreground uppercase text-[11px] tracking-wider", "Raw ER7 Pipe-Delimited Frame" }
                                        pre { class: "p-3 rounded-xl bg-muted/60 border border-border font-mono text-[11px] text-foreground overflow-x-auto whitespace-pre-wrap", "{raw_er7}" }
                                    }
                                    div { class: "space-y-1.5",
                                        label { class: "font-bold text-foreground uppercase text-[11px] tracking-wider", "Generated MLLP ACK Response Frame" }
                                        pre { class: "p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20 font-mono text-[11px] text-emerald-600 dark:text-emerald-400 overflow-x-auto whitespace-pre-wrap", "{ack_payload}" }
                                    }
                                    div { class: "space-y-1.5",
                                        label { class: "font-bold text-foreground uppercase text-[11px] tracking-wider", "Parsed Structured JSON Tree" }
                                        pre { class: "p-3 rounded-xl bg-muted/60 border border-border font-mono text-[11px] text-foreground overflow-x-auto whitespace-pre-wrap", "{parsed_json}" }
                                    }
                                }
                                div { class: "flex items-center justify-end pt-3 border-t border-border",
                                    button {
                                        class: "px-4 py-2 text-xs font-semibold rounded-xl bg-secondary text-secondary-foreground hover:bg-secondary/80 cursor-pointer border border-border",
                                        onclick: move |_| selected_hl7_msg.set(None),
                                        "Close Inspector"
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
