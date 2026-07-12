use crate::components;
use dioxus::prelude::*;
use yntra_core::{TodoItem, ZeroCopyStore, P2PMeshSyncRouter};
use std::sync::Arc;

#[component]
pub fn P2PPlaygroundView(active_user_id: Signal<String>, db_trigger: Signal<u32>) -> Element {
    // 1. Initialize Peer stores and mesh router
    let store_a = use_hook(|| {
        let path = std::env::temp_dir().join("yntra_zero_copy_peer_a.db").to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path); // Clean start
        ZeroCopyStore::new(path).expect("Failed to initialize Peer A Store")
    }).clone();

    let store_b = use_hook(|| {
        let path = std::env::temp_dir().join("yntra_zero_copy_peer_b.db").to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path); // Clean start
        ZeroCopyStore::new(path).expect("Failed to initialize Peer B Store")
    }).clone();

    let router = use_hook(|| {
        P2PMeshSyncRouter::with_relay("http://localhost:8081".to_string())
    }).clone();

    // 2. Local reactive state
    let mut todos_a = use_signal(Vec::<TodoItem>::new);
    let mut todos_b = use_signal(Vec::<TodoItem>::new);
    let mut input_a = use_signal(String::new);
    let mut input_b = use_signal(String::new);
    let mut logs = use_signal(|| vec![
        "[P2P Mesh] Collaboration Playground started.".to_string(),
        "[P2P Mesh] Connecting peers to local coordination relay server (port 8081)...".to_string()
    ]);
    let mut is_connected = use_signal(|| true);

    // Register peers on network and load initial lists
    use_effect({
        let store_a = store_a.clone();
        let store_b = store_b.clone();
        let router = router.clone();
        move || {
            router.register_peer_network("peer_a".to_string());
            router.register_peer_network("peer_b".to_string());

            if let Ok(list) = store_a.read_all_todos() {
                todos_a.set(list);
            }
            if let Ok(list) = store_b.read_all_todos() {
                todos_b.set(list);
            }
        }
    });

    // Real-time network sync polling loop
    use_effect({
        let store_a = store_a.clone();
        let store_b = store_b.clone();
        let router = router.clone();
        let is_connected = is_connected.clone();
        move || {
            let store_a = store_a.clone();
            let store_b = store_b.clone();
            let router = router.clone();
            let is_connected = is_connected.clone();
            spawn(async move {
                loop {
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    
                    #[cfg(target_arch = "wasm32")]
                    {
                        let promise = js_sys::Promise::new(&mut |resolve, _| {
                            let window = web_sys::window().unwrap();
                            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1500);
                        });
                        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    }

                    if *is_connected.read() {
                        router.trigger_poll_relay_updates("peer_a".to_string(), Arc::new(store_a.clone()));
                        router.trigger_poll_relay_updates("peer_b".to_string(), Arc::new(store_b.clone()));
                    }

                    if let Ok(list) = store_a.read_all_todos() {
                        todos_a.set(list);
                    }
                    if let Ok(list) = store_b.read_all_todos() {
                        todos_b.set(list);
                    }
                }
            });
        }
    });

    rsx! {
        div { class: "flex flex-col h-full w-full bg-background box-border p-6",
            
            // Header bar
            div { class: "flex shrink-0 items-center justify-between border-b border-border pb-4 mb-6",
                div {
                    h1 { class: "text-2xl font-bold text-foreground m-0 flex items-center gap-3",
                        components::LucideIcon { name: "refresh-cw", size: "24" }
                        "P2P Collaboration Sync Playground"
                    }
                    p { class: "text-muted-foreground text-sm m-0 mt-1",
                        "Real-time local synchronization using memory-mapped ZeroCopyStores and Loro CRDT merge packets."
                    }
                }
                
                // Network status indicator
                div { class: "flex items-center gap-3 bg-white/[0.02] border border-border px-4 py-2 rounded-lg",
                    span { class: "text-sm font-semibold", "Peer Mesh Link:" }
                    button {
                        class: if *is_connected.read() {
                            "yntra-btn success flex items-center gap-2 bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 px-3 py-1 rounded-md text-xs cursor-pointer"
                        } else {
                            "yntra-btn danger flex items-center gap-2 bg-rose-500/10 text-rose-400 border border-rose-500/20 px-3 py-1 rounded-md text-xs cursor-pointer"
                        },
                        onclick: move |_| {
                            let curr = *is_connected.read();
                            is_connected.set(!curr);
                            let mut curr_logs = logs.read().clone();
                            curr_logs.push(format!("[P2P Mesh] Link switched to {}", if !curr { "ONLINE" } else { "OFFLINE" }));
                            logs.set(curr_logs);
                        },
                        components::LucideIcon { name: if *is_connected.read() { "wifi" } else { "wifi-off" }, size: "14" }
                        if *is_connected.read() { "Online (Connected)" } else { "Offline (Link Severed)" }
                    }
                }
            }

            // Grid layout for Peer Nodes A & B
            div { class: "grid grid-cols-2 gap-6 flex-1 min-h-0 mb-6",
                
                // Peer A panel
                div { class: "flex flex-col border border-border rounded-xl bg-card overflow-hidden",
                    div { class: "p-4 border-b border-border bg-white/[0.01] flex items-center justify-between",
                        div { class: "flex items-center gap-2",
                            span { class: "w-2.5 h-2.5 rounded-full bg-blue-500 animate-pulse" }
                            h3 { class: "text-lg font-bold text-foreground m-0", "Peer Node A" }
                        }
                        span { class: "text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded", "store: yntra_zero_copy_peer_a.db" }
                    }
                    
                    // Input bar
                    div { class: "p-4 border-b border-border flex gap-2",
                        input {
                            class: "flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm text-foreground focus:outline-none focus:border-primary",
                            placeholder: "Add item local to Peer A...",
                            value: "{input_a}",
                            oninput: move |e| input_a.set(e.value().clone()),
                            onkeydown: {
                                let store = store_a.clone();
                                let router = router.clone();
                                move |e| {
                                    if e.key() == keyboard_types::Key::Enter {
                                        let text = input_a.read().clone();
                                        if text.trim().is_empty() { return; }
                                        let mut list = store.read_all_todos().unwrap_or_default();
                                        list.push(TodoItem {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            workspace_id: "ws-p2p".to_string(),
                                            text: text.clone(),
                                            completed: false,
                                            updated_at: yntra_core::infra::time::get_current_time_ms(),
                                            sync_status: "pending".to_string(),
                                        });
                                        let _ = store.write_todos(list);
                                        input_a.set(String::new());
                                        if let Ok(list) = store.read_all_todos() {
                                            todos_a.set(list.clone());
                                        }
                                        if let Ok(changes) = store.get_loro_changes() {
                                            router.broadcast_write_network("peer_a".to_string(), changes);
                                        }
                                        let mut curr_logs = logs.read().clone();
                                        curr_logs.push(format!("[Peer A] Created and broadcasted local item: \"{}\"", text));
                                        logs.set(curr_logs);
                                    }
                                }
                            }
                        }
                        button {
                            class: "yntra-btn primary bg-primary text-primary-foreground border-0 px-4 py-2 rounded-lg text-sm cursor-pointer hover:opacity-90 flex items-center gap-1",
                            onclick: {
                                let store = store_a.clone();
                                let router = router.clone();
                                move |_| {
                                    let text = input_a.read().clone();
                                    if text.trim().is_empty() { return; }
                                    let mut list = store.read_all_todos().unwrap_or_default();
                                    list.push(TodoItem {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        workspace_id: "ws-p2p".to_string(),
                                        text: text.clone(),
                                        completed: false,
                                        updated_at: yntra_core::infra::time::get_current_time_ms(),
                                        sync_status: "pending".to_string(),
                                    });
                                    let _ = store.write_todos(list);
                                    input_a.set(String::new());
                                    if let Ok(list) = store.read_all_todos() {
                                        todos_a.set(list.clone());
                                    }
                                    if let Ok(changes) = store.get_loro_changes() {
                                        router.broadcast_write_network("peer_a".to_string(), changes);
                                    }
                                    let mut curr_logs = logs.read().clone();
                                    curr_logs.push(format!("[Peer A] Created and broadcasted local item: \"{}\"", text));
                                    logs.set(curr_logs);
                                }
                            },
                            "Add"
                        }
                    }

                    // Scrollable list
                    div { class: "flex-1 overflow-y-auto p-4 flex flex-col gap-2",
                        if todos_a.read().is_empty() {
                            div { class: "text-center text-muted-foreground text-sm py-8", "No items local to Node A" }
                        } else {
                            for t in todos_a.read().clone() {
                                {
                                    let id = t.id.clone();
                                    rsx! {
                                        div { class: "flex items-center justify-between bg-white/[0.01] border border-border/40 p-3 rounded-lg hover:border-border transition",
                                            div { class: "flex items-center gap-3",
                                                input {
                                                    type: "checkbox",
                                                    checked: t.completed,
                                                    class: "cursor-pointer",
                                                    onclick: {
                                                        let store = store_a.clone();
                                                        let id_toggle = id.clone();
                                                        let router = router.clone();
                                                        move |_| {
                                                            let mut list = store.read_all_todos().unwrap_or_default();
                                                            for todo in list.iter_mut() {
                                                                if todo.id == id_toggle {
                                                                    todo.completed = !todo.completed;
                                                                    todo.updated_at = yntra_core::infra::time::get_current_time_ms();
                                                                }
                                                            }
                                                            let _ = store.write_todos(list);
                                                            if let Ok(list) = store.read_all_todos() {
                                                                todos_a.set(list.clone());
                                                            }
                                                            if let Ok(changes) = store.get_loro_changes() {
                                                                router.broadcast_write_network("peer_a".to_string(), changes);
                                                            }
                                                            let mut curr_logs = logs.read().clone();
                                                            curr_logs.push("[Peer A] Toggled item state & broadcasted".to_string());
                                                            logs.set(curr_logs);
                                                        }
                                                    }
                                                }
                                                span {
                                                    class: if t.completed { "text-muted-foreground line-through" } else { "text-foreground" },
                                                    "{t.text}"
                                                }
                                            }
                                            span { class: "text-[10px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded", "Pending Sync" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Local sync trigger
                    div { class: "p-4 border-t border-border bg-white/[0.01] flex justify-end",
                        button {
                            class: "yntra-btn flex items-center gap-2 bg-blue-500/10 text-blue-400 border border-blue-500/20 px-4 py-2 rounded-lg text-sm cursor-pointer hover:bg-blue-500/20 transition",
                            onclick: {
                                let store_a = store_a.clone();
                                let store_b = store_b.clone();
                                let router = router.clone();
                                move |_| {
                                    if !*is_connected.read() {
                                        let mut curr_logs = logs.read().clone();
                                        curr_logs.push("[P2P Mesh] Sync failed: Peer link is offline".to_string());
                                        logs.set(curr_logs);
                                        return;
                                    }
                                    if let Ok(changes) = store_a.get_loro_changes() {
                                        let len = changes.len();
                                        let mut curr_logs = logs.read().clone();
                                        curr_logs.push(format!("[Peer A] Exported Loro update ({} bytes)", len));
                                        
                                        router.broadcast_write_network("peer_a".to_string(), changes);
                                        curr_logs.push("[P2P Router] Broadcasted update to relay server".to_string());
                                        
                                        router.trigger_poll_relay_updates("peer_b".to_string(), Arc::new(store_b.clone()));
                                        curr_logs.push("[Peer B] Polled and merged CRDT changes successfully!".to_string());
                                        if let Ok(list) = store_b.read_all_todos() {
                                            todos_b.set(list);
                                        }
                                        logs.set(curr_logs);
                                    }
                                }
                            },
                            components::LucideIcon { name: "arrow-right", size: "14" }
                            "Sync A -> B"
                        }
                    }
                }

                // Peer B panel
                div { class: "flex flex-col border border-border rounded-xl bg-card overflow-hidden",
                    div { class: "p-4 border-b border-border bg-white/[0.01] flex items-center justify-between",
                        div { class: "flex items-center gap-2",
                            span { class: "w-2.5 h-2.5 rounded-full bg-purple-500 animate-pulse" }
                            h3 { class: "text-lg font-bold text-foreground m-0", "Peer Node B" }
                        }
                        span { class: "text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded", "store: yntra_zero_copy_peer_b.db" }
                    }
                    
                    // Input bar
                    div { class: "p-4 border-b border-border flex gap-2",
                        input {
                            class: "flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm text-foreground focus:outline-none focus:border-primary",
                            placeholder: "Add item local to Peer B...",
                            value: "{input_b}",
                            oninput: move |e| input_b.set(e.value().clone()),
                            onkeydown: {
                                let store = store_b.clone();
                                let router = router.clone();
                                move |e| {
                                    if e.key() == keyboard_types::Key::Enter {
                                        let text = input_b.read().clone();
                                        if text.trim().is_empty() { return; }
                                        let mut list = store.read_all_todos().unwrap_or_default();
                                        list.push(TodoItem {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            workspace_id: "ws-p2p".to_string(),
                                            text: text.clone(),
                                            completed: false,
                                            updated_at: yntra_core::infra::time::get_current_time_ms(),
                                            sync_status: "pending".to_string(),
                                        });
                                        let _ = store.write_todos(list);
                                        input_b.set(String::new());
                                        if let Ok(list) = store.read_all_todos() {
                                            todos_b.set(list.clone());
                                        }
                                        if let Ok(changes) = store.get_loro_changes() {
                                            router.broadcast_write_network("peer_b".to_string(), changes);
                                        }
                                        let mut curr_logs = logs.read().clone();
                                        curr_logs.push(format!("[Peer B] Created and broadcasted local item: \"{}\"", text));
                                        logs.set(curr_logs);
                                    }
                                }
                            }
                        }
                        button {
                            class: "yntra-btn primary bg-primary text-primary-foreground border-0 px-4 py-2 rounded-lg text-sm cursor-pointer hover:opacity-90 flex items-center gap-1",
                            onclick: {
                                let store = store_b.clone();
                                let router = router.clone();
                                move |_| {
                                    let text = input_b.read().clone();
                                    if text.trim().is_empty() { return; }
                                    let mut list = store.read_all_todos().unwrap_or_default();
                                    list.push(TodoItem {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        workspace_id: "ws-p2p".to_string(),
                                        text: text.clone(),
                                        completed: false,
                                        updated_at: yntra_core::infra::time::get_current_time_ms(),
                                        sync_status: "pending".to_string(),
                                    });
                                    let _ = store.write_todos(list);
                                    input_b.set(String::new());
                                    if let Ok(list) = store.read_all_todos() {
                                        todos_b.set(list.clone());
                                    }
                                    if let Ok(changes) = store.get_loro_changes() {
                                        router.broadcast_write_network("peer_b".to_string(), changes);
                                    }
                                    let mut curr_logs = logs.read().clone();
                                    curr_logs.push(format!("[Peer B] Created and broadcasted local item: \"{}\"", text));
                                    logs.set(curr_logs);
                                }
                            },
                            "Add"
                        }
                    }

                    // Scrollable list
                    div { class: "flex-1 overflow-y-auto p-4 flex flex-col gap-2",
                        if todos_b.read().is_empty() {
                            div { class: "text-center text-muted-foreground text-sm py-8", "No items local to Node B" }
                        } else {
                            for t in todos_b.read().clone() {
                                {
                                    let id = t.id.clone();
                                    rsx! {
                                        div { class: "flex items-center justify-between bg-white/[0.01] border border-border/40 p-3 rounded-lg hover:border-border transition",
                                            div { class: "flex items-center gap-3",
                                                input {
                                                    type: "checkbox",
                                                    checked: t.completed,
                                                    class: "cursor-pointer",
                                                    onclick: {
                                                        let store = store_b.clone();
                                                        let id_toggle = id.clone();
                                                        let router = router.clone();
                                                        move |_| {
                                                            let mut list = store.read_all_todos().unwrap_or_default();
                                                            for todo in list.iter_mut() {
                                                                if todo.id == id_toggle {
                                                                    todo.completed = !todo.completed;
                                                                    todo.updated_at = yntra_core::infra::time::get_current_time_ms();
                                                                }
                                                            }
                                                            let _ = store.write_todos(list);
                                                            if let Ok(list) = store.read_all_todos() {
                                                                todos_b.set(list.clone());
                                                            }
                                                            if let Ok(changes) = store.get_loro_changes() {
                                                                router.broadcast_write_network("peer_b".to_string(), changes);
                                                            }
                                                            let mut curr_logs = logs.read().clone();
                                                            curr_logs.push("[Peer B] Toggled item state & broadcasted".to_string());
                                                            logs.set(curr_logs);
                                                        }
                                                    }
                                                }
                                                span {
                                                    class: if t.completed { "text-muted-foreground line-through" } else { "text-foreground" },
                                                    "{t.text}"
                                                }
                                            }
                                            span { class: "text-[10px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded", "Pending Sync" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Local sync trigger
                    div { class: "p-4 border-t border-border bg-white/[0.01] flex justify-between items-center",
                        button {
                            class: "yntra-btn flex items-center gap-2 bg-purple-500/10 text-purple-400 border border-purple-500/20 px-4 py-2 rounded-lg text-sm cursor-pointer hover:bg-purple-500/20 transition",
                            onclick: {
                                let store_a = store_a.clone();
                                let store_b = store_b.clone();
                                let router = router.clone();
                                move |_| {
                                    if !*is_connected.read() {
                                        let mut curr_logs = logs.read().clone();
                                        curr_logs.push("[P2P Mesh] Sync failed: Peer link is offline".to_string());
                                        logs.set(curr_logs);
                                        return;
                                    }
                                    if let Ok(changes) = store_b.get_loro_changes() {
                                        let len = changes.len();
                                        let mut curr_logs = logs.read().clone();
                                        curr_logs.push(format!("[Peer B] Exported Loro update ({} bytes)", len));
                                        
                                        router.broadcast_write_network("peer_b".to_string(), changes);
                                        curr_logs.push("[P2P Router] Broadcasted update to relay server".to_string());
                                        
                                        router.trigger_poll_relay_updates("peer_a".to_string(), Arc::new(store_a.clone()));
                                        curr_logs.push("[Peer A] Polled and merged CRDT changes successfully!".to_string());
                                        if let Ok(list) = store_a.read_all_todos() {
                                            todos_a.set(list);
                                        }
                                        logs.set(curr_logs);
                                    }
                                }
                            },
                            components::LucideIcon { name: "arrow-left", size: "14" }
                            "Sync B -> A"
                        }
                    }
                }
            }

            // Sync log console at bottom
            div { class: "border border-border rounded-xl bg-muted/20 overflow-hidden flex flex-col h-48",
                div { class: "p-3 border-b border-border bg-white/[0.01] flex justify-between items-center shrink-0",
                    h4 { class: "text-sm font-bold text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "terminal", size: "14" }
                        "P2P Sync Connection Logs"
                    }
                    button {
                        class: "yntra-btn text-muted-foreground hover:text-foreground text-xs bg-transparent border-0 cursor-pointer",
                        onclick: {
                            let store_a = store_a.clone();
                            let store_b = store_b.clone();
                            move |_| {
                                let _ = store_a.write_todos(Vec::new());
                                let _ = store_b.write_todos(Vec::new());
                                if let Ok(list) = store_a.read_all_todos() {
                                    todos_a.set(list);
                                }
                                if let Ok(list) = store_b.read_all_todos() {
                                    todos_b.set(list);
                                }
                                logs.set(vec!["[P2P Mesh] Stores cleared.".to_string()]);
                            }
                        },
                        "Clear Stores"
                    }
                }
                div { class: "flex-1 overflow-y-auto p-4 font-mono text-xs flex flex-col gap-1.5 text-emerald-400/90",
                    for l in logs.read().iter() {
                        div { "{l}" }
                    }
                }
            }
        }
    }
}
