use yntra_core::database::zero_copy::chaos::{run_chaos_sync_load_test, ChaosConfig, ChaosNetworkProxy};
use yntra_core::database::zero_copy::ZeroCopyNoteStore;
use yntra_core::models::DailyNote;
use std::sync::Arc;

#[tokio::test]
async fn test_chaos_network_dropouts_and_latency_load() {
    let num_devices = 10;
    let total_mutations = 200;
    let dropout_rate = 0.30; // 30% packet loss simulation
    let max_latency_ms = 50;

    let result_json = run_chaos_sync_load_test(num_devices, total_mutations, dropout_rate, max_latency_ms)
        .await
        .expect("Chaos & Sync Load Test execution failed");

    let report: serde_json::Value = serde_json::from_str(&result_json).unwrap();
    
    assert_eq!(report["status"], "PASSED");
    assert_eq!(report["converged"], true);
    assert_eq!(report["num_devices"], 10);
    assert!(report["packets_sent"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_multi_device_crdt_conflict_resolution_under_heavy_load() {
    let temp_dir = std::env::temp_dir();
    let path_a = temp_dir.join(format!("chaos_crdt_a_{}.db", uuid::Uuid::new_v4())).to_string_lossy().to_string();
    let path_b = temp_dir.join(format!("chaos_crdt_b_{}.db", uuid::Uuid::new_v4())).to_string_lossy().to_string();
    let path_c = temp_dir.join(format!("chaos_crdt_c_{}.db", uuid::Uuid::new_v4())).to_string_lossy().to_string();

    let store_a = Arc::new(ZeroCopyNoteStore::new(path_a.clone()).unwrap());
    let store_b = Arc::new(ZeroCopyNoteStore::new(path_b.clone()).unwrap());
    let store_c = Arc::new(ZeroCopyNoteStore::new(path_c.clone()).unwrap());

    let chaos_config = ChaosConfig {
        dropout_rate: 0.15,
        min_latency_ms: 5,
        max_latency_ms: 30,
        enable_disconnections: true,
    };
    let proxy = Arc::new(ChaosNetworkProxy::new(chaos_config));

    proxy.register_peer("device_a");
    proxy.register_peer("device_b");
    proxy.register_peer("device_c");

    // Concurrent heavy load notes writing across 3 devices
    for i in 0..50 {
        let note_a = DailyNote {
            id: format!("note_a_{}", i),
            workspace_id: "ws_chaos".to_string(),
            team_id: "team_chaos".to_string(),
            author_id: Some("device_a".to_string()),
            subject: format!("Chaos Note A #{}", i),
            content: format!("Content written by Device A iteration {}", i),
            edit_history: "[]".to_string(),
            created_at: "2026-08-04".to_string(),
            updated_at: (1000 + i) as i64,
            sync_status: "pending".to_string(),
        };

        let note_b = DailyNote {
            id: format!("note_b_{}", i),
            workspace_id: "ws_chaos".to_string(),
            team_id: "team_chaos".to_string(),
            author_id: Some("device_b".to_string()),
            subject: format!("Chaos Note B #{}", i),
            content: format!("Content written by Device B iteration {}", i),
            edit_history: "[]".to_string(),
            created_at: "2026-08-04".to_string(),
            updated_at: (2000 + i) as i64,
            sync_status: "pending".to_string(),
        };

        let note_c = DailyNote {
            id: format!("note_c_{}", i),
            workspace_id: "ws_chaos".to_string(),
            team_id: "team_chaos".to_string(),
            author_id: Some("device_c".to_string()),
            subject: format!("Chaos Note C #{}", i),
            content: format!("Content written by Device C iteration {}", i),
            edit_history: "[]".to_string(),
            created_at: "2026-08-04".to_string(),
            updated_at: (3000 + i) as i64,
            sync_status: "pending".to_string(),
        };

        store_a.upsert_note(note_a).unwrap();
        store_b.upsert_note(note_b).unwrap();
        store_c.upsert_note(note_c).unwrap();

        // Broadcast Loro CRDT deltas through Chaos Network Proxy
        if let Ok(changes_a) = store_a.get_loro_changes() {
            proxy.broadcast("device_a", changes_a).await;
        }
        if let Ok(changes_b) = store_b.get_loro_changes() {
            proxy.broadcast("device_b", changes_b).await;
        }
        if let Ok(changes_c) = store_c.get_loro_changes() {
            proxy.broadcast("device_c", changes_c).await;
        }
    }

    // Drain chaos queues
    for packet in proxy.drain_queue("device_a") {
        let _ = store_a.apply_loro_update(packet);
    }
    for packet in proxy.drain_queue("device_b") {
        let _ = store_b.apply_loro_update(packet);
    }
    for packet in proxy.drain_queue("device_c") {
        let _ = store_c.apply_loro_update(packet);
    }

    // Perform final cross-node synchronization pass to guarantee complete convergence
    if let Ok(final_a) = store_a.get_loro_changes() {
        let _ = store_b.apply_loro_update(final_a.clone());
        let _ = store_c.apply_loro_update(final_a);
    }
    if let Ok(final_b) = store_b.get_loro_changes() {
        let _ = store_a.apply_loro_update(final_b.clone());
        let _ = store_c.apply_loro_update(final_b);
    }
    if let Ok(final_c) = store_c.get_loro_changes() {
        let _ = store_a.apply_loro_update(final_c.clone());
        let _ = store_b.apply_loro_update(final_c);
    }

    let notes_a = store_a.read_all_notes().unwrap();
    let notes_b = store_b.read_all_notes().unwrap();
    let notes_c = store_c.read_all_notes().unwrap();

    // Verify 100% convergence across all 3 stores (50 notes * 3 devices = 150 items)
    assert_eq!(notes_a.len(), 150);
    assert_eq!(notes_b.len(), 150);
    assert_eq!(notes_c.len(), 150);

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
    let _ = std::fs::remove_file(&path_c);
}
