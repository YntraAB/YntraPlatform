use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yntra_core::{
    delete_mllp_listener, encode_mllp_frame, get_hl7_messages, get_mllp_listeners,
    process_raw_hl7_v2_message, save_mllp_listener, start_mllp_listener, stop_mllp_listener,
    MllpListenerConfig, MLLP_END_BLOCK_1, MLLP_END_BLOCK_2, MLLP_START_BLOCK,
};

#[tokio::test]
async fn test_mllp_listener_full_lifecycle_and_socket_stream() {
    let _lock = yntra_core::database::DB_TEST_LOCK.lock().unwrap();

    let test_dir = std::env::temp_dir().join(format!("mllp_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&test_dir);
    yntra_core::set_database_directory(test_dir.to_str().unwrap().to_string()).unwrap();

    let conn = yntra_core::database::acquire_connection().await.unwrap();
    yntra_core::database::setup_schema(&conn).await.unwrap();
    conn.execute("UPDATE users SET role = 'platform_admin' WHERE id = 'user-1'", ()).await.unwrap();

    let admin_id = "user-1";
    let ws_id = "workspace-1";

    // 1. Save new MLLP listener config
    let config = MllpListenerConfig {
        id: "mllp-test-listener-1".to_string(),
        workspace_id: ws_id.to_string(),
        name: "Test ADT MLLP Port 25755".to_string(),
        port: 25755,
        bind_address: "127.0.0.1".to_string(),
        tls_enabled: false,
        status: "stopped".to_string(),
        last_active_at: 0,
        created_at: 0,
        updated_at: 0,
    };

    let saved = save_mllp_listener(admin_id.to_string(), config).await.unwrap();
    assert_eq!(saved.port, 25755);
    assert_eq!(saved.status, "stopped");

    // 2. Start listener
    let started = start_mllp_listener(admin_id.to_string(), ws_id.to_string(), saved.id.clone())
        .await
        .unwrap();
    assert_eq!(started.status, "running");

    // Wait a brief moment for socket binding
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // 3. Connect TCP client and transmit MLLP framed HL7 ADT^A01 message
    let raw_hl7 = "MSH|^~\\&|EPIC_ADT|HOSPITAL_A|YNTRA_EMR|YNTRA_CLINIC|20260806120000||ADT^A01|MSG_INT_101|P|2.3\rPID|1||MRN_TEST_999^^^HOSP||SMITH^JOHN^A||19850412|M|||100 MAIN ST^^NEW YORK^NY^10001\rPV1|1|I|ICU^BED-01^ROOM-101";
    let mllp_bytes = encode_mllp_frame(raw_hl7);

    let mut stream = tokio::net::TcpStream::connect("127.0.0.1:25755")
        .await
        .expect("Failed to connect to MLLP socket server");

    stream.write_all(&mllp_bytes).await.expect("Failed to send MLLP frame");

    // 4. Read MLLP ACK response from server
    let mut ack_buf = vec![0u8; 1024];
    let n = stream.read(&mut ack_buf).await.expect("Failed to read ACK response");
    assert!(n > 3);
    assert_eq!(ack_buf[0], MLLP_START_BLOCK);
    assert_eq!(ack_buf[n - 2], MLLP_END_BLOCK_1);
    assert_eq!(ack_buf[n - 1], MLLP_END_BLOCK_2);

    let ack_str = String::from_utf8_lossy(&ack_buf[1..n - 2]);
    assert!(ack_str.contains("MSA|AA|MSG_INT_101"));

    // 5. Verify database persistence in hl7_messages table
    let messages = get_hl7_messages(admin_id.to_string(), ws_id.to_string(), Some(saved.id.clone()), Some(10))
        .await
        .unwrap();

    assert!(!messages.is_empty());
    let rec = &messages[0];
    assert_eq!(rec.message_control_id, "MSG_INT_101");
    assert_eq!(rec.message_type, "ADT");
    assert_eq!(rec.trigger_event, "A01");
    assert_eq!(rec.patient_mrn.as_deref(), Some("MRN_TEST_999"));
    assert_eq!(rec.patient_name.as_deref(), Some("JOHN A, SMITH"));
    assert_eq!(rec.ack_status, "AA");

    // 6. Test manual payload ingestion API
    let manual_hl7 = "MSH|^~\\&|LAB_SYS|LAB_FAC|YNTRA|YNTRA_FAC|20260806130000||ORU^R01|MSG_INT_102|P|2.3\rPID|1||MRN_TEST_888||DOE^JANE\rOBR|1|ORD100|LAB200|CBC^COMPLETE BLOOD COUNT\rOBX|1|NM|WBC^WHITE BLOOD CELL COUNT||6.8|10^3/uL|4.5-11.0|N|||F";
    let manual_rec = process_raw_hl7_v2_message(ws_id.to_string(), None, manual_hl7.to_string())
        .await
        .unwrap();
    assert_eq!(manual_rec.message_control_id, "MSG_INT_102");
    assert_eq!(manual_rec.message_type, "ORU");

    // 7. Stop listener
    let stopped = stop_mllp_listener(admin_id.to_string(), ws_id.to_string(), saved.id.clone())
        .await
        .unwrap();
    assert_eq!(stopped.status, "stopped");

    // 8. Delete listener
    let deleted = delete_mllp_listener(admin_id.to_string(), ws_id.to_string(), saved.id.clone())
        .await
        .unwrap();
    assert!(deleted);

    let listeners = get_mllp_listeners(admin_id.to_string(), ws_id.to_string()).await.unwrap();
    assert!(listeners.is_empty());
}
