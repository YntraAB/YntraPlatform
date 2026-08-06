use yntra_core::{
    export_condition_to_fhir_r4, export_encounter_to_fhir_r4, export_observation_to_fhir_r4,
    export_patient_to_fhir_r4, export_workspace_fhir_bundle, get_fhir_resource_mappings,
    import_fhir_r4_resource, validate_fhir_r4_payload,
};

#[tokio::test]
async fn test_fhir_r4_engine_full_export_import_and_bundle_lifecycle() {
    let _lock = yntra_core::database::DB_TEST_LOCK.lock().unwrap();

    let test_dir = std::env::temp_dir().join(format!("fhir_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&test_dir);
    yntra_core::set_database_directory(test_dir.to_str().unwrap().to_string()).unwrap();

    let conn = yntra_core::database::acquire_connection().await.unwrap();
    yntra_core::database::setup_schema(&conn).await.unwrap();

    let admin_id = "user-1";
    let ws_id = "workspace-1";
    let client_id = "cli-test-101";

    conn.execute(
        "INSERT INTO clients (id, workspace_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status) \
         VALUES (?1, ?2, 'EMMA', 'WATSON', 'MRN-998877', 'High Support Needed', '{\"gender\": \"female\"}', '2026-08-06', 0, 'synced')",
        yntra_core::params![client_id, ws_id],
    )
    .await
    .unwrap();

    // Insert sample job ticket into job_tickets table
    let ticket_id = "ticket-test-202";
    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, created_at, updated_at, sync_status) \
         VALUES (?1, ?2, 'Annual Health Assessment', 'Routine Assessment', 'Clinical Suite A', 'high', 'completed', ?3, '2026-08-06', '[]', '2026-08-06', 0, 'synced')",
        yntra_core::params![ticket_id, ws_id, admin_id],
    )
    .await
    .unwrap();

    // Insert sample note into notes table
    let note_id = "note-test-303";
    conn.execute(
        "INSERT INTO notes (id, workspace_id, team_id, author_id, subject, content, created_at, updated_at, sync_status) \
         VALUES (?1, ?2, 'team-1', ?3, 'Vital Signs & Exam Note', 'Patient resting comfortably. BP 120/80, HR 72 bpm.', '2026-08-06', 0, 'synced')",
        yntra_core::params![note_id, ws_id, admin_id],
    )
    .await
    .unwrap();

    // 1. Test Export Patient to FHIR R4
    let patient_res = export_patient_to_fhir_r4(admin_id.to_string(), ws_id.to_string(), client_id.to_string())
        .await
        .unwrap();
    assert_eq!(patient_res.resource_type, "Patient");
    assert_eq!(patient_res.fhir_id, format!("p-{}", client_id));
    assert!(patient_res.validation_passed);
    assert!(patient_res.json_payload.contains("\"resourceType\": \"Patient\""));
    assert!(patient_res.json_payload.contains("MRN-998877"));
    assert!(patient_res.json_payload.contains("\"gender\": \"female\""));
    assert!(!patient_res.json_payload.contains("high support needed"));

    // 2. Test Export Encounter to FHIR R4
    let enc_res = export_encounter_to_fhir_r4(admin_id.to_string(), ws_id.to_string(), ticket_id.to_string())
        .await
        .unwrap();
    assert_eq!(enc_res.resource_type, "Encounter");
    assert_eq!(enc_res.fhir_id, format!("enc-{}", ticket_id));
    assert!(enc_res.json_payload.contains("finished"));
    assert!(enc_res.json_payload.contains("Patient/p-cli-test-101"));
    assert!(!enc_res.json_payload.contains("Patient/p-user-1"));

    // 3. Test Export Observation to FHIR R4
    let obs_res = export_observation_to_fhir_r4(admin_id.to_string(), ws_id.to_string(), note_id.to_string())
        .await
        .unwrap();
    assert_eq!(obs_res.resource_type, "Observation");
    assert!(obs_res.json_payload.contains("11506-3"));

    // 4. Test Export Condition to FHIR R4
    let cond_res = export_condition_to_fhir_r4(admin_id.to_string(), ws_id.to_string(), "cond-909".to_string())
        .await
        .unwrap();
    assert_eq!(cond_res.resource_type, "Condition");
    assert!(cond_res.json_payload.contains("I10"));

    // 5. Test Workspace FHIR R4 Bundle Export
    let bundle_json = export_workspace_fhir_bundle(admin_id.to_string(), ws_id.to_string(), Some("collection".to_string()))
        .await
        .unwrap();
    let (bundle_type, _) = validate_fhir_r4_payload(&bundle_json).unwrap();
    assert_eq!(bundle_type, "Bundle");
    assert!(bundle_json.contains("\"total\": 4"));

    // 6. Test Import External FHIR R4 Patient JSON
    let external_fhir_patient = r#"{
        "resourceType": "Patient",
        "id": "ext-pat-555",
        "active": true,
        "name": [{ "text": "DAVID MILLER", "family": "MILLER", "given": ["DAVID"] }],
        "identifier": [{ "system": "MRN", "value": "MRN-555123" }]
    }"#;

    let import_res = import_fhir_r4_resource(admin_id.to_string(), ws_id.to_string(), external_fhir_patient.to_string())
        .await
        .unwrap();
    assert_eq!(import_res.resource_type, "Patient");
    assert!(import_res.success);

    // 7. Verify Mapping Records in Database
    let mappings = get_fhir_resource_mappings(admin_id.to_string(), ws_id.to_string())
        .await
        .unwrap();
    assert_eq!(mappings.len(), 5);

    // 8. Test EHR FHIR Endpoint Validation & Pending Sync Engine
    let unconfigured_ehr = yntra_core::EhrIntegrationConfig {
        id: "ehr-test-unconfigured".to_string(),
        workspace_id: ws_id.to_string(),
        provider: "Epic".to_string(),
        fhir_endpoint_url: "unconfigured".to_string(),
        account_id: None,
        api_token: None,
        refresh_token: None,
        token_expires_at: 0,
        mtls_client_cert_pem: None,
        mtls_client_key_pem: None,
        sync_direction: "two_way".to_string(),
        auto_sync_enabled: true,
        last_synced_at: 0,
        sync_status: "idle".to_string(),
        error_message: None,
        sync_token: None,
        created_at: 0,
        updated_at: 0,
    };
    yntra_core::save_ehr_integration(admin_id.to_string(), unconfigured_ehr).await.unwrap();

    let err_res = yntra_core::sync_ehr_fhir_records(admin_id.to_string(), ws_id.to_string(), "ehr-test-unconfigured".to_string()).await;
    assert!(err_res.is_err());

    let configured_ehr = yntra_core::EhrIntegrationConfig {
        id: "ehr-test-active".to_string(),
        workspace_id: ws_id.to_string(),
        provider: "Epic CareEverywhere".to_string(),
        fhir_endpoint_url: "https://fhir.epic.com/interconnect-fhir-oauth2/api/FHIR/R4".to_string(),
        account_id: None,
        api_token: Some("test-token".to_string()),
        refresh_token: None,
        token_expires_at: 0,
        mtls_client_cert_pem: None,
        mtls_client_key_pem: None,
        sync_direction: "two_way".to_string(),
        auto_sync_enabled: true,
        last_synced_at: 0,
        sync_status: "idle".to_string(),
        error_message: None,
        sync_token: None,
        created_at: 0,
        updated_at: 0,
    };
    yntra_core::save_ehr_integration(admin_id.to_string(), configured_ehr).await.unwrap();

    let sync_res = yntra_core::sync_ehr_fhir_records(admin_id.to_string(), ws_id.to_string(), "ehr-test-active".to_string()).await.unwrap();
    assert_eq!(sync_res.status, "success");
}
