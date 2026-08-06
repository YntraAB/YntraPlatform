use yntra_core::{
    cancel_ncpdp_prescription, create_ncpdp_new_rx_prescription, get_ncpdp_prescriptions,
    parse_and_import_inbound_ncpdp_xml, parse_ncpdp_script_xml,
    transmit_ncpdp_script_to_surescripts, validate_ncpdp_script_payload,
};

#[tokio::test]
async fn test_ncpdp_script_eprescribing_full_lifecycle() {
    let _lock = yntra_core::database::DB_TEST_LOCK.lock().unwrap();

    let test_dir = std::env::temp_dir().join(format!("ncpdp_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&test_dir);
    yntra_core::set_database_directory(test_dir.to_str().unwrap().to_string()).unwrap();

    let conn = yntra_core::database::acquire_connection().await.unwrap();
    yntra_core::database::setup_schema(&conn).await.unwrap();

    let admin_id = "user-1";
    let ws_id = "workspace-1";
    let client_id = "cli-ncpdp-100";

    // Insert sample client record
    conn.execute(
        "INSERT INTO clients (id, workspace_id, first_name, last_name, personal_number, created_at, updated_at, sync_status) \
         VALUES (?1, ?2, 'SARAH', 'CONNOR', 'MRN-445566', '2026-08-06', 0, 'synced')",
        yntra_core::params![client_id, ws_id],
    )
    .await
    .unwrap();

    let prescriber_npi = "1992003004";
    let pharmacy_npi = "1881002003";

    // 1. Test Input Validation Failures
    assert!(validate_ncpdp_script_payload("123", pharmacy_npi, "Amoxicillin", 30.0, 10).is_err());
    assert!(validate_ncpdp_script_payload(prescriber_npi, "99", "Amoxicillin", 30.0, 10).is_err());
    assert!(validate_ncpdp_script_payload(prescriber_npi, pharmacy_npi, "", 30.0, 10).is_err());
    assert!(validate_ncpdp_script_payload(prescriber_npi, pharmacy_npi, "Amoxicillin", 0.0, 10).is_err());

    // 2. Create NewRx Electronic Prescription
    let create_res = create_ncpdp_new_rx_prescription(
        admin_id.to_string(),
        ws_id.to_string(),
        client_id.to_string(),
        prescriber_npi.to_string(),
        pharmacy_npi.to_string(),
        Some("Walgreens Pharmacy #104".to_string()),
        "Amoxicillin 500mg Oral Capsule".to_string(),
        Some("RxNorm-308182".to_string()),
        Some("NDC-00093-3109-05".to_string()),
        30.0,
        10,
        2,
        "Take 1 capsule by mouth three times daily for 10 days".to_string(),
    )
    .await
    .unwrap();

    assert!(create_res.success);
    assert_eq!(create_res.status, "draft");
    assert_eq!(create_res.transaction_type, "NewRx");
    assert!(create_res.xml_payload.contains("Amoxicillin 500mg Oral Capsule"));
    assert!(create_res.xml_payload.contains(prescriber_npi));
    assert!(create_res.xml_payload.contains(pharmacy_npi));

    let rx_id = create_res.prescription_id.clone();

    // 3. Transmit to Surescripts Network
    let tx_res = transmit_ncpdp_script_to_surescripts(
        admin_id.to_string(),
        ws_id.to_string(),
        rx_id.clone(),
        None,
    )
    .await
    .unwrap();

    assert!(tx_res.success);
    assert_eq!(tx_res.status, "transmitted");
    assert!(tx_res.surescripts_tx_id.starts_with("SURE-TX-"));

    // 4. Cancel Prescription (CancelRx)
    let cancel_res = cancel_ncpdp_prescription(
        admin_id.to_string(),
        ws_id.to_string(),
        rx_id.clone(),
        "Dosage adjusted by attending physician".to_string(),
    )
    .await
    .unwrap();

    assert!(cancel_res.success);
    assert_eq!(cancel_res.status, "cancelled");
    assert!(cancel_res.xml_payload.contains("CancelRx"));

    // 5. Query Prescriptions List
    let rx_list = get_ncpdp_prescriptions(admin_id.to_string(), ws_id.to_string(), Some(client_id.to_string()))
        .await
        .unwrap();
    assert_eq!(rx_list.len(), 1);
    assert_eq!(rx_list[0].id, rx_id);
    assert_eq!(rx_list[0].status, "cancelled");

    // 6. Test Inbound Pharmacy Renewal Request XML Import
    let inbound_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Message version="v2017071" xmlns="http://www.ncpdp.org/schema/SCRIPT">
  <Header>
    <To Qualifier="P">1992003004</To>
    <From Qualifier="C">1881002003</From>
    <MessageID>msg-inbound-999</MessageID>
    <RelatesToMessageID>rx-orig-100</RelatesToMessageID>
  </Header>
  <Body>
    <RxRenewalRequest>
      <StoreName>CVS Pharmacy #4401</StoreName>
      <MedicationPrescribed>
        <DrugDescription>Lisinopril 10mg Oral Tablet</DrugDescription>
        <QuantityValue>90</QuantityValue>
        <DaysSupply>90</DaysSupply>
        <Refills>3</Refills>
        <Sig>Take 1 tablet daily</Sig>
      </MedicationPrescribed>
    </RxRenewalRequest>
  </Body>
</Message>"#;

    let parsed_tx = parse_ncpdp_script_xml(inbound_xml).unwrap();
    assert_eq!(parsed_tx.transaction_type, "RxRenewalRequest");
    assert_eq!(parsed_tx.medication.drug_name, "Lisinopril 10mg Oral Tablet");

    let import_res = parse_and_import_inbound_ncpdp_xml(admin_id.to_string(), ws_id.to_string(), inbound_xml.to_string())
        .await
        .unwrap();
    assert!(import_res.success);
    assert_eq!(import_res.status, "received");

    let all_rx = get_ncpdp_prescriptions(admin_id.to_string(), ws_id.to_string(), None)
        .await
        .unwrap();
    assert_eq!(all_rx.len(), 2);
}
