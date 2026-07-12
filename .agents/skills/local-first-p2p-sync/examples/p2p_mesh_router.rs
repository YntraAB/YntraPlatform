use yntra_core::{P2PMeshSyncRouter, ZeroCopyStore, TodoItem};

// Example showing P2P mesh network configuration and state syncing
pub fn run_p2p_sync_example(peer_id: String, relay_url: String) -> Result<(), String> {
    // 1. Initialize local store
    let path = std::env::temp_dir().join(format!("yntra_store_{}.db", peer_id)).to_string_lossy().to_string();
    let store = ZeroCopyStore::new(path).map_err(|e| e.to_string())?;

    // 2. Initialize P2P mesh sync router
    let router = P2PMeshSyncRouter::with_relay(relay_url);

    // 3. Register peer to the signaling/coordination network
    router.register_peer_network(peer_id.clone());

    // 4. Simulate a write and broadcast event
    let new_todo = TodoItem {
        id: "todo-101".to_string(),
        workspace_id: "workspace-1".to_string(),
        title: "Buy groceries".to_string(),
        completed: false,
        updated_at: 1690000000,
    };

    // Save locally
    store.write_todos(vec![new_todo]).map_err(|e| e.to_string())?;

    // Broadcast the update payload to other mesh peers
    let update_payload = vec![1, 2, 3, 4]; // Mock serialized binary update log (e.g. Loro/Automerge log)
    router.broadcast_update(update_payload);

    println!("Peer {} successfully broadcasted update to mesh.", peer_id);
    Ok(())
}
