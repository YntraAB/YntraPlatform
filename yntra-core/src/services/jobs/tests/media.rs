use crate::database;
use crate::services::jobs::{
    create_job_ticket, enqueue_offline_media_blob, get_offline_media_pointer,
    sync_pending_offline_media_blobs,
};

#[tokio::test]
async fn test_offline_media_crdt_optimization() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup test workspace and user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-media-test', 'Media Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-media-crew', 'ws-media-test', 'crew@media.io', 'admin')", ()).await.unwrap();

    // 2. Create job ticket
    let job = create_job_ticket(
        "u-media-crew".to_string(),
        "ws-media-test".to_string(),
        "Basement Move Media Test".to_string(),
        "High res inspection photo in basement".to_string(),
        "Basement St 5".to_string(),
        "high".to_string(),
        None,
        "2026-09-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // 3. Enqueue heavy 1.5MB base64 inspection photo blob
    let heavy_payload = "A".repeat(1_500_000); // 1.5MB mock photo data string
    let pointer = enqueue_offline_media_blob(
        "u-media-crew".to_string(),
        job.id.clone(),
        "photo".to_string(),
        heavy_payload.clone(),
    )
    .await
    .unwrap();

    // Verify SHA-256 content pointer generation and compression stats
    assert!(pointer.hash_pointer.starts_with("sha256:"));
    assert_eq!(pointer.media_type, "photo");
    assert_eq!(pointer.original_size_bytes, 1_500_000);
    assert!(pointer.compressed_size_bytes < 1_500_000);
    assert!(pointer.compression_ratio_percent > 50.0);
    assert_eq!(pointer.upload_status, "queued_offline");

    // 4. Retrieve media pointer via SHA-256 hash
    let fetched =
        get_offline_media_pointer("u-media-crew".to_string(), pointer.hash_pointer.clone())
            .await
            .unwrap();
    assert!(fetched.is_some());
    let f = fetched.unwrap();
    assert_eq!(f.hash_pointer, pointer.hash_pointer);
    assert_eq!(f.upload_status, "queued_offline");

    // 5. Simulate cell network recovery and process background upload queue
    let synced_count = sync_pending_offline_media_blobs("u-media-crew".to_string(), job.id.clone())
        .await
        .unwrap();
    assert_eq!(synced_count, 1);

    let fetched_synced =
        get_offline_media_pointer("u-media-crew".to_string(), pointer.hash_pointer.clone())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(fetched_synced.upload_status, "synced");

    // Cleanup
    conn.execute(
        "DELETE FROM offline_media_blobs WHERE workspace_id = 'ws-media-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-media-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-media-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-media-test'", ())
        .await
        .unwrap();
}
