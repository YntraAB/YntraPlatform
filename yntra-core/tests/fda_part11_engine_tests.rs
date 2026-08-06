use yntra_core::{
    execute_fda_part11_dual_signature, get_fda_part11_signatures,
    verify_fda_part11_dual_signature,
};

#[tokio::test]
async fn test_fda_part11_dual_person_electronic_signatures() {
    let _lock = yntra_core::database::DB_TEST_LOCK.lock().unwrap();

    let test_dir = std::env::temp_dir().join(format!("fda_part11_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&test_dir);
    yntra_core::set_database_directory(test_dir.to_str().unwrap().to_string()).unwrap();

    let conn = yntra_core::database::acquire_connection().await.unwrap();
    yntra_core::database::setup_schema(&conn).await.unwrap();

    let admin_id = "user-1";
    let supervisor_id = "user-2";
    let ws_id = "workspace-1";
    let chart_record_id = "chart-rev-9001";

    // 1. Test Input Validation Rejections
    assert!(execute_fda_part11_dual_signature(
        admin_id.to_string(),
        ws_id.to_string(),
        "clinical_chart".to_string(),
        chart_record_id.to_string(),
        "".to_string(), // Empty printed name
        "Authorship".to_string(),
        None,
        None,
        None,
    )
    .await
    .is_err());

    assert!(execute_fda_part11_dual_signature(
        admin_id.to_string(),
        ws_id.to_string(),
        "clinical_chart".to_string(),
        chart_record_id.to_string(),
        "Dr. Alice Smith".to_string(),
        "".to_string(), // Empty intent
        None,
        None,
        None,
    )
    .await
    .is_err());

    // 2. Execute Single-Signer Signature (Primary Clinician)
    let single_res = execute_fda_part11_dual_signature(
        admin_id.to_string(),
        ws_id.to_string(),
        "lab_approval".to_string(),
        "lab-100".to_string(),
        "Dr. Alice Smith, MD".to_string(),
        "Lab Result Verification".to_string(),
        None,
        None,
        None,
    )
    .await
    .unwrap();

    assert!(single_res.is_valid);
    assert!(single_res.primary_verified);
    assert!(!single_res.dual_sign_completed);

    let verify_single = verify_fda_part11_dual_signature(
        admin_id.to_string(),
        ws_id.to_string(),
        single_res.signature_id.clone(),
    )
    .await
    .unwrap();
    assert!(verify_single.is_valid);
    assert!(verify_single.primary_verified);

    // 3. Execute Dual-Person Signature (Primary Authorizer + Secondary Supervisor Cosigner)
    let dual_res = execute_fda_part11_dual_signature(
        admin_id.to_string(),
        ws_id.to_string(),
        "controlled_prescription".to_string(),
        "rx-schedule2-550".to_string(),
        "Dr. Alice Smith, MD".to_string(),
        "Prescribing Schedule II Controlled Substance".to_string(),
        Some(supervisor_id.to_string()),
        Some("Dr. Robert Vance, MD (Chief Medical Officer)".to_string()),
        Some("Cosigning & Clinical Supervision Approval".to_string()),
    )
    .await
    .unwrap();

    assert!(dual_res.is_valid);
    assert!(dual_res.dual_sign_completed);
    assert!(dual_res.primary_verified);
    assert!(dual_res.secondary_verified);

    // 4. Verify Dual Signature Cryptographic Integrity
    let verify_dual = verify_fda_part11_dual_signature(
        admin_id.to_string(),
        ws_id.to_string(),
        dual_res.signature_id.clone(),
    )
    .await
    .unwrap();

    assert!(verify_dual.is_valid);
    assert!(verify_dual.dual_sign_completed);
    assert!(verify_dual.primary_verified);
    assert!(verify_dual.secondary_verified);

    // 5. Query FDA Electronic Signatures Audit History
    let sig_logs = get_fda_part11_signatures(admin_id.to_string(), ws_id.to_string(), None)
        .await
        .unwrap();
    assert_eq!(sig_logs.len(), 2);
    assert!(sig_logs.iter().any(|s| s.dual_sign_completed));
}
